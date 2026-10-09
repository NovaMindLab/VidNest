use crate::db::Database;
use crate::downloader::executor::{
    build_download_args, execute_download, ingest_downloaded_video, DownloadOutcome,
    TaskControlSignal,
};
use crate::downloader::parser::{extract_platform_from_url, ProgressUpdate};
use crate::models::DownloadTask;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::{Mutex, RwLock, Semaphore};

static MANAGER_INSTANCE: OnceLock<Arc<DownloadManager>> = OnceLock::new();
static TASK_COUNTER: AtomicU64 = AtomicU64::new(1);

pub struct DownloadManager {
    tasks: Arc<RwLock<HashMap<String, DownloadTask>>>,
    controllers: Arc<Mutex<HashMap<String, tokio::sync::watch::Sender<TaskControlSignal>>>>,
    semaphore: Arc<Semaphore>,
}

impl Default for DownloadManager {
    fn default() -> Self {
        Self::new(3)
    }
}

impl DownloadManager {
    pub fn new(max_concurrent: usize) -> Self {
        Self {
            tasks: Arc::new(RwLock::new(HashMap::new())),
            controllers: Arc::new(Mutex::new(HashMap::new())),
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
        }
    }

    /// Accesses the global singleton DownloadManager instance.
    pub fn global() -> Arc<Self> {
        MANAGER_INSTANCE
            .get_or_init(|| Arc::new(Self::default()))
            .clone()
    }

    /// Loads previous tasks from SQLite into the in-memory pool.
    pub async fn load_tasks_from_db(&self, db: &Database) -> Result<(), String> {
        let conn_arc = db.get_connection();
        let conn = conn_arc.lock().map_err(|e| e.to_string())?;
        let tasks = crate::db::tasks::load_all_download_tasks(&conn)?;

        let mut lock = self.tasks.write().await;
        for task in tasks {
            lock.insert(task.id.clone(), task);
        }

        Ok(())
    }

    /// Starts a new video download task.
    pub async fn start_download(
        &self,
        url: String,
        format_id: Option<String>,
        app_data_dir: PathBuf,
        db: Database,
    ) -> Result<DownloadTask, String> {
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;

        let count = TASK_COUNTER.fetch_add(1, Ordering::SeqCst);
        let task_id = format!("dl_{}_{:x}", now_ms, count);

        let platform = extract_platform_from_url(&url);

        let task = DownloadTask {
            id: task_id.clone(),
            url: url.clone(),
            title: None,
            author: None,
            platform: Some(platform),
            format_id: format_id.clone(),
            status: "pending".to_string(),
            progress: 0.0,
            downloaded_bytes: 0,
            total_bytes: 0,
            speed: 0.0,
            speed_str: None,
            eta: None,
            file_path: None,
            error_log: None,
            created_at: now_ms,
            updated_at: now_ms,
        };

        // Save into memory
        {
            let mut tasks_lock = self.tasks.write().await;
            tasks_lock.insert(task_id.clone(), task.clone());
        }

        // Persist into SQLite
        {
            let conn_arc = db.get_connection();
            let conn = conn_arc.lock().map_err(|e| e.to_string())?;
            let _ = crate::db::tasks::insert_download_task(&conn, &task);
        }

        // Setup control signal channel
        let (tx, rx) = tokio::sync::watch::channel(TaskControlSignal::Run);
        {
            let mut ctrl_lock = self.controllers.lock().await;
            ctrl_lock.insert(task_id.clone(), tx);
        }

        // Spawn background execution task
        self.spawn_task_worker(task_id.clone(), url, format_id, app_data_dir, db, rx);

        Ok(task)
    }

    /// Pauses an active download task.
    pub async fn pause_task(&self, task_id: &str) -> Result<bool, String> {
        // Send pause signal if running
        let ctrl_lock = self.controllers.lock().await;
        if let Some(tx) = ctrl_lock.get(task_id) {
            let _ = tx.send(TaskControlSignal::Pause);
        }

        let mut tasks_lock = self.tasks.write().await;
        if let Some(task) = tasks_lock.get_mut(task_id) {
            if task.status == "downloading" || task.status == "pending" {
                task.status = "paused".to_string();
                task.updated_at = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as i64;
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Resumes a paused or errored download task.
    pub async fn resume_task(
        &self,
        task_id: &str,
        app_data_dir: PathBuf,
        db: Database,
    ) -> Result<bool, String> {
        let (url, format_id) = {
            let mut tasks_lock = self.tasks.write().await;
            let task = match tasks_lock.get_mut(task_id) {
                Some(t) => t,
                None => return Err(format!("未找到任务: {}", task_id)),
            };

            if task.status == "downloading" {
                return Ok(true); // Already running
            }

            task.status = "pending".to_string();
            task.updated_at = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as i64;

            (task.url.clone(), task.format_id.clone())
        };

        // Create new control signal channel
        let (tx, rx) = tokio::sync::watch::channel(TaskControlSignal::Run);
        {
            let mut ctrl_lock = self.controllers.lock().await;
            ctrl_lock.insert(task_id.to_string(), tx);
        }

        // Spawn worker
        self.spawn_task_worker(task_id.to_string(), url, format_id, app_data_dir, db, rx);

        Ok(true)
    }

    /// Cancels a download task.
    pub async fn cancel_task(&self, task_id: &str) -> Result<bool, String> {
        // Send cancel signal
        {
            let ctrl_lock = self.controllers.lock().await;
            if let Some(tx) = ctrl_lock.get(task_id) {
                let _ = tx.send(TaskControlSignal::Cancel);
            }
        }

        let mut tasks_lock = self.tasks.write().await;
        if let Some(task) = tasks_lock.get_mut(task_id) {
            task.status = "cancelled".to_string();
            task.updated_at = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as i64;
            return Ok(true);
        }

        Ok(false)
    }

    /// Retrieves all download tasks, ordered by created_at DESC.
    pub async fn get_tasks(&self) -> Vec<DownloadTask> {
        let lock = self.tasks.read().await;
        let mut list: Vec<DownloadTask> = lock.values().cloned().collect();
        list.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        list
    }

    /// Clears completed and cancelled tasks from memory and database.
    pub async fn clear_completed(&self) -> usize {
        let mut removed_count = 0;
        let mut lock = self.tasks.write().await;
        lock.retain(|_, task| {
            if task.status == "completed" || task.status == "cancelled" {
                removed_count += 1;
                false
            } else {
                true
            }
        });

        removed_count
    }

    /// Helper to update a task's progress in memory and SQLite.
    pub async fn update_task_progress(&self, task_id: &str, update: ProgressUpdate) {
        let mut lock = self.tasks.write().await;
        if let Some(task) = lock.get_mut(task_id) {
            if let Some(p) = update.progress {
                task.progress = p;
            }
            if let Some(tb) = update.total_bytes {
                task.total_bytes = tb;
            }
            if let Some(db) = update.downloaded_bytes {
                task.downloaded_bytes = db;
            }
            if let Some(s) = update.speed {
                task.speed = s;
            }
            if let Some(ss) = update.speed_str {
                task.speed_str = Some(ss);
            }
            if let Some(e) = update.eta {
                task.eta = Some(e);
            }
            if let Some(fp) = update.file_path {
                task.file_path = Some(fp);
            }
            task.updated_at = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as i64;
        }
    }

    /// Spawns the background task worker that respects concurrency limit and executes download.
    fn spawn_task_worker(
        &self,
        task_id: String,
        url: String,
        format_id: Option<String>,
        app_data_dir: PathBuf,
        db: Database,
        mut control_rx: tokio::sync::watch::Receiver<TaskControlSignal>,
    ) {
        let tasks_arc = self.tasks.clone();
        let semaphore_arc = self.semaphore.clone();

        tokio::spawn(async move {
            // Wait for semaphore permit (limits concurrent downloads)
            let _permit = tokio::select! {
                res = semaphore_arc.acquire() => match res {
                    Ok(p) => p,
                    Err(_) => return,
                },
                _ = control_rx.changed() => {
                    let sig = *control_rx.borrow();
                    if sig == TaskControlSignal::Cancel || sig == TaskControlSignal::Pause {
                        return;
                    }
                    match semaphore_arc.acquire().await {
                        Ok(p) => p,
                        Err(_) => return,
                    }
                }
            };

            // Check if task cancelled/paused while waiting in queue
            {
                let sig = *control_rx.borrow();
                if sig == TaskControlSignal::Cancel || sig == TaskControlSignal::Pause {
                    return;
                }
            }

            // Set status to downloading
            {
                let mut lock = tasks_arc.write().await;
                if let Some(t) = lock.get_mut(&task_id) {
                    if t.status == "cancelled" || t.status == "paused" {
                        return;
                    }
                    t.status = "downloading".to_string();
                    t.updated_at = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis() as i64;
                }
            }

            // Resolve yt-dlp binary
            let (active_bin, _) = crate::ytdlp::resolve_active_binary(Some(&app_data_dir));
            let bin_path = match active_bin {
                Some(b) => b,
                None => {
                    let mut lock = tasks_arc.write().await;
                    if let Some(t) = lock.get_mut(&task_id) {
                        t.status = "error".to_string();
                        t.error_log = Some("未找到 yt-dlp 可执行文件".to_string());
                    }
                    return;
                }
            };

            // Setup download directory and output template
            let downloads_dir = app_data_dir.join("downloads");
            let _ = tokio::fs::create_dir_all(&downloads_dir).await;
            let output_template = downloads_dir
                .join("%(title).100s [%(id)s].%(ext)s")
                .to_string_lossy()
                .to_string();

            let args = build_download_args(&url, format_id.as_deref(), &output_template);

            let tasks_for_progress = tasks_arc.clone();
            let tid = task_id.clone();
            let on_progress = move |update: ProgressUpdate| {
                let tasks_ref = tasks_for_progress.clone();
                let task_key = tid.clone();
                tokio::spawn(async move {
                    let mut lock = tasks_ref.write().await;
                    if let Some(task) = lock.get_mut(&task_key) {
                        if let Some(p) = update.progress {
                            task.progress = p;
                        }
                        if let Some(tb) = update.total_bytes {
                            task.total_bytes = tb;
                        }
                        if let Some(db) = update.downloaded_bytes {
                            task.downloaded_bytes = db;
                        }
                        if let Some(s) = update.speed {
                            task.speed = s;
                        }
                        if let Some(ss) = update.speed_str {
                            task.speed_str = Some(ss);
                        }
                        if let Some(e) = update.eta {
                            task.eta = Some(e);
                        }
                        if let Some(fp) = update.file_path {
                            task.file_path = Some(fp);
                        }
                        task.updated_at = SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_millis() as i64;
                    }
                });
            };

            let outcome = execute_download(
                &bin_path,
                &args,
                &downloads_dir,
                control_rx,
                on_progress,
            )
            .await;

            let now_ms = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as i64;

            match outcome {
                DownloadOutcome::Success { file_path } => {
                    let (title, author, platform) = {
                        let lock = tasks_arc.read().await;
                        let t = lock.get(&task_id);
                        (
                            t.and_then(|x| x.title.clone()),
                            t.and_then(|x| x.author.clone()),
                            t.and_then(|x| x.platform.clone()),
                        )
                    };

                    // Run automatic DB ingestion closed-loop
                    let ingest_res = ingest_downloaded_video(
                        &file_path,
                        &task_id,
                        &url,
                        title.as_deref(),
                        author.as_deref(),
                        platform.as_deref(),
                        &db,
                        &app_data_dir,
                    );

                    let mut lock = tasks_arc.write().await;
                    if let Some(t) = lock.get_mut(&task_id) {
                        match ingest_res {
                            Ok(_) => {
                                t.status = "completed".to_string();
                                t.progress = 100.0;
                                t.file_path = Some(file_path.to_string_lossy().to_string());
                                t.updated_at = now_ms;
                            }
                            Err(e) => {
                                t.status = "error".to_string();
                                t.error_log = Some(format!("入库失败: {}", e));
                                t.updated_at = now_ms;
                            }
                        }

                        // Sync to SQLite
                        sync_task_to_db(&db, t);
                    }
                }
                DownloadOutcome::Paused => {
                    let mut lock = tasks_arc.write().await;
                    if let Some(t) = lock.get_mut(&task_id) {
                        t.status = "paused".to_string();
                        t.updated_at = now_ms;
                        sync_task_to_db(&db, t);
                    }
                }
                DownloadOutcome::Cancelled => {
                    let mut lock = tasks_arc.write().await;
                    if let Some(t) = lock.get_mut(&task_id) {
                        t.status = "cancelled".to_string();
                        t.updated_at = now_ms;
                        sync_task_to_db(&db, t);
                    }
                }
                DownloadOutcome::Failed { error } => {
                    let mut lock = tasks_arc.write().await;
                    if let Some(t) = lock.get_mut(&task_id) {
                        t.status = "error".to_string();
                        t.error_log = Some(error);
                        t.updated_at = now_ms;
                        sync_task_to_db(&db, t);
                    }
                }
            }
        });
    }
}

fn sync_task_to_db(db: &Database, task: &DownloadTask) {
    let conn_arc = db.get_connection();
    let lock_res = conn_arc.lock();
    if let Ok(conn) = lock_res {
        let _ = crate::db::tasks::update_download_task(&conn, task);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_manager_lifecycle() {
        let manager = DownloadManager::new(3);
        let db = Database::init_in_memory().expect("in-memory db");
        let temp_dir = std::env::temp_dir();

        let task = manager
            .start_download(
                "https://www.youtube.com/watch?v=dummy".to_string(),
                None,
                temp_dir,
                db,
            )
            .await
            .expect("start download");

        assert_eq!(task.status, "pending");
        assert_eq!(task.progress, 0.0);

        let tasks = manager.get_tasks().await;
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].id, task.id);

        let paused = manager.pause_task(&task.id).await.expect("pause");
        assert!(paused);

        let cancelled = manager.cancel_task(&task.id).await.expect("cancel");
        assert!(cancelled);

        let cleared = manager.clear_completed().await;
        assert_eq!(cleared, 1);

        let tasks_after = manager.get_tasks().await;
        assert_eq!(tasks_after.len(), 0);
    }
}
