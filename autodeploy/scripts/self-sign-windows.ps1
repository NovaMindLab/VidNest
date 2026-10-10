<#
.SYNOPSIS
    VidNest - Industrial-grade Windows Code Signing & Authenticode Timestamp Automation Script
.DESCRIPTION
    Creates or imports code signing certificates, generates standard PFX files,
    and applies Authenticode signatures using signtool.exe with official RFC 3161
    timestamps (e.g. DigiCert, Sectigo).

    Crucial for SmartScreen reputation accumulation:
    RFC 3161 countersignatures prove that the binary was signed when the certificate
    was valid, allowing Windows SmartScreen to accumulate reputation across version updates
    without recalculating hash trust from scratch.

.PARAMETER TargetBinary
    The executable, installer (.exe), MSI or DLL file to sign.
.PARAMETER Action
    Workflow action:
    - 'FullPipeline'    : Create cert (if missing) -> Export PFX -> Sign TargetBinary -> Verify
    - 'CreateCertOnly'  : Only create self-signed certificate and export to PFX
    - 'SignOnly'        : Sign TargetBinary using existing PFX certificate
    - 'TrustCertLocally': Install certificate into Local Trusted Root Authorities (dev machines)
.PARAMETER Subject
    Subject CN / Organization for the certificate. Default: "CN=NovaMindLab, O=NovaMindLab, OU=VidNest Desktop"
.PARAMETER PfxPath
    Path where the .pfx file is stored or loaded. Default: ".\certs\NovaMindLab_CodeSign.pfx"
.PARAMETER Password
    Password securing the PFX container. Default: "VidNestCodeSign2026!"
.PARAMETER TimestampUrl
    Primary RFC 3161 timestamp authority. Default: "http://timestamp.digicert.com"
.PARAMETER SigntoolPath
    Explicit path to signtool.exe. If omitted, the script auto-detects from Windows Kits/VS/PATH.
.PARAMETER TrustLocally
    Switch to also import the generated certificate into CurrentUser\Root for zero-warning testing.
.PARAMETER Verify
    Switch to run Authenticode verification after signing (Default: $true).

.EXAMPLE
    .\autodeploy\scripts\self-sign-windows.ps1 -TargetBinary ".\dist\Local.Video.Intelligence_0.1.0_x64-setup.exe"

.EXAMPLE
    .\autodeploy\scripts\self-sign-windows.ps1 -Action CreateCertOnly -PfxPath ".\certs\VidNest.pfx"

.EXAMPLE
    .\autodeploy\scripts\self-sign-windows.ps1 -Action SignOnly -PfxPath ".\certs\VidNest.pfx" -TargetBinary ".\LocalVideoIntelligence.exe"
#>

[CmdletBinding()]
param(
    [Parameter(Position = 0, Mandatory = $false)]
    [string]$TargetBinary = "",

    [ValidateSet('FullPipeline', 'CreateCertOnly', 'SignOnly', 'TrustCertLocally')]
    [string]$Action = "FullPipeline",

    [string]$Subject = "CN=NovaMindLab, O=NovaMindLab, OU=VidNest Desktop",
    [string]$PfxPath = ".\certs\NovaMindLab_CodeSign.pfx",
    [string]$Password = "VidNestCodeSign2026!",
    [string]$TimestampUrl = "http://timestamp.digicert.com",
    [string[]]$FallbackTimestampUrls = @(
        "http://timestamp.sectigo.com",
        "http://timestamp.globalsign.com/tsa/r6advanced1",
        "http://tsa.starfieldtech.com"
    ),
    [string]$SigntoolPath = "",
    [switch]$TrustLocally = $false,
    [bool]$Verify = $true
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

Write-Host "╔══════════════════════════════════════════════════════════════╗" -ForegroundColor Cyan
Write-Host "║    VidNest Windows Authenticode Code Signing Specialist      ║" -ForegroundColor Cyan
Write-Host "╚══════════════════════════════════════════════════════════════╝" -ForegroundColor Cyan

# -----------------------------------------------------------------------------
# 1. Helper: Locate signtool.exe
# -----------------------------------------------------------------------------
function Find-Signtool {
    if ($SigntoolPath -and (Test-Path $SigntoolPath)) {
        return $SigntoolPath
    }

    # Check system PATH
    $cmd = Get-Command "signtool.exe" -ErrorAction SilentlyContinue
    if ($cmd) {
        return $cmd.Source
    }

    # Search Windows Kits & SDKs
    $sdkSearchPaths = @(
        "${env:ProgramFiles(x86)}\Windows Kits\10\bin\*\x64\signtool.exe",
        "${env:ProgramFiles}\Windows Kits\10\bin\*\x64\signtool.exe",
        "${env:ProgramFiles(x86)}\Windows Kits\10\App Certification Kit\signtool.exe",
        "${env:ProgramFiles}\Microsoft Visual Studio\*\*\bin\Hostx64\x64\signtool.exe"
    )

    foreach ($pattern in $sdkSearchPaths) {
        $found = Get-Item $pattern -ErrorAction SilentlyContinue | Sort-Object FullName -Descending | Select-Object -First 1
        if ($found) {
            return $found.FullName
        }
    }

    return $null
}

# -----------------------------------------------------------------------------
# 2. Helper: Create Self-Signed Code Signing Certificate
# -----------------------------------------------------------------------------
function New-CodeSignCertificate {
    param([string]$CertSubject)

    Write-Host "`n🔑 Generating self-signed Code Signing Certificate..." -ForegroundColor Yellow
    Write-Host "   Subject: $CertSubject"

    $cert = New-SelfSignedCertificate `
        -Type CodeSigningCert `
        -Subject $CertSubject `
        -CertStoreLocation "Cert:\CurrentUser\My" `
        -KeyLength 4096 `
        -HashAlgorithm SHA256 `
        -NotAfter (Get-Date).AddYears(5) `
        -KeyUsage DigitalSignature `
        -KeyExportPolicy Exportable `
        -FriendlyName "NovaMindLab VidNest Code Signing"

    Write-Host "   Thumbprint : $($cert.Thumbprint)" -ForegroundColor Green
    Write-Host "   Valid Until: $($cert.NotAfter.ToString('yyyy-MM-dd'))" -ForegroundColor Green

    return $cert
}

# -----------------------------------------------------------------------------
# 3. Helper: Export Certificate to PFX
# -----------------------------------------------------------------------------
function Export-PfxFile {
    param(
        [System.Security.Cryptography.X509Certificates.X509Certificate2]$Certificate,
        [string]$Path,
        [string]$Pwd
    )

    $parentDir = Split-Path -Parent $Path
    if ($parentDir -and -not (Test-Path $parentDir)) {
        New-Item -ItemType Directory -Path $parentDir -Force | Out-Null
    }

    $securePwd = ConvertTo-SecureString -String $Pwd -Force -AsPlainText
    Export-PfxCertificate -Cert $Certificate -FilePath $Path -Password $securePwd | Out-Null
    Write-Host "✅ Exported PFX certificate to: $Path" -ForegroundColor Green
}

# -----------------------------------------------------------------------------
# 4. Helper: Trust Certificate Locally
# -----------------------------------------------------------------------------
function Install-TrustLocally {
    param([string]$Path, [string]$Pwd)

    Write-Host "`n🛡️  Installing certificate into CurrentUser\Root (Trusted Root)..." -ForegroundColor Yellow
    $securePwd = ConvertTo-SecureString -String $Pwd -Force -AsPlainText
    $cert = New-Object System.Security.Cryptography.X509Certificates.X509Certificate2
    $cert.Import($Path, $securePwd, [System.Security.Cryptography.X509Certificates.X509KeyStorageFlags]::PersistKeySet)

    $store = New-Object System.Security.Cryptography.X509Certificates.X509Store(
        [System.Security.Cryptography.X509Certificates.StoreName]::Root,
        [System.Security.Cryptography.X509Certificates.StoreLocation]::CurrentUser
    )
    $store.Open([System.Security.Cryptography.X509Certificates.OpenFlags]::ReadWrite)
    $store.Add($cert)
    $store.Close()

    Write-Host "✅ Certificate trusted locally in CurrentUser\Root!" -ForegroundColor Green
}

# -----------------------------------------------------------------------------
# 5. Helper: Sign Target Binary with RFC 3161 Timestamp
# -----------------------------------------------------------------------------
function Invoke-CodeSign {
    param(
        [string]$ToolPath,
        [string]$FileToSign,
        [string]$Pfx,
        [string]$Pwd,
        [string]$PrimaryTs,
        [string[]]$FallbackTs
    )

    if (-not (Test-Path $FileToSign)) {
        throw "Target binary to sign does not exist: $FileToSign"
    }

    Write-Host "`n✍️  Signing binary: $FileToSign" -ForegroundColor Yellow
    Write-Host "   Using signtool: $ToolPath"

    $tsList = @($PrimaryTs) + $FallbackTs
    $signSuccess = $false

    foreach ($ts in $tsList) {
        Write-Host "   Attempting RFC 3161 timestamp via: $ts ..." -ForegroundColor Gray
        $signArgs = @(
            "sign",
            "/f", $Pfx,
            "/p", $Pwd,
            "/fd", "SHA256",
            "/tr", $ts,
            "/td", "SHA256",
            "/d", "Local Video Intelligence",
            "/du", "https://github.com/NovaMindLab/VidNest",
            $FileToSign
        )

        $proc = Start-Process -FilePath $ToolPath -ArgumentList $signArgs -NoNewWindow -Wait -PassThru
        if ($proc.ExitCode -eq 0) {
            Write-Host "✅ Successfully signed and timestamped with $ts!" -ForegroundColor Green
            $signSuccess = $true
            break
        } else {
            Write-Host "⚠️  Timestamp server $ts failed (ExitCode: $($proc.ExitCode)). Trying next fallback..." -ForegroundColor Yellow
        }
    }

    if (-not $signSuccess) {
        throw "Failed to sign binary using any of the provided RFC 3161 timestamp servers."
    }
}

# -----------------------------------------------------------------------------
# 6. Helper: Verify Signature
# -----------------------------------------------------------------------------
function Test-CodeSignature {
    param([string]$ToolPath, [string]$FileToCheck)

    Write-Host "`n🔍 Verifying Authenticode signature on: $FileToCheck" -ForegroundColor Yellow

    # Method A: signtool verify
    $verifyArgs = @("verify", "/pa", "/v", $FileToCheck)
    $proc = Start-Process -FilePath $ToolPath -ArgumentList $verifyArgs -NoNewWindow -Wait -PassThru
    if ($proc.ExitCode -ne 0) {
        Write-Host "⚠️  signtool verify reported warnings or untrusted root (expected for self-signed unless installed in Root store)." -ForegroundColor Yellow
    } else {
        Write-Host "✓ signtool verify passed!" -ForegroundColor Green
    }

    # Method B: PowerShell Get-AuthenticodeSignature
    $authSig = Get-AuthenticodeSignature -FilePath $FileToCheck
    Write-Host "   Status        : $($authSig.Status)"
    Write-Host "   Status Message: $($authSig.StatusMessage)"
    if ($authSig.SignerCertificate) {
        Write-Host "   Signer Subject: $($authSig.SignerCertificate.Subject)"
        Write-Host "   Thumbprint    : $($authSig.SignerCertificate.Thumbprint)"
    }
    if ($authSig.TimeStamperCertificate) {
        Write-Host "   RFC 3161 TSA  : $($authSig.TimeStamperCertificate.Subject)" -ForegroundColor Cyan
    }
}

# -----------------------------------------------------------------------------
# 7. Execution Orchestration
# -----------------------------------------------------------------------------

# Step A: Locate signtool
$resolvedSigntool = Find-Signtool
if (-not $resolvedSigntool -and ($Action -in @('FullPipeline', 'SignOnly'))) {
    Write-Host "⚠️  signtool.exe was not detected on this host." -ForegroundColor Yellow
    Write-Host "   Please ensure Windows 10/11 SDK or Visual Studio Build Tools are installed." -ForegroundColor Yellow
    Write-Host "   Or pass explicit path via: -SigntoolPath 'C:\path\to\signtool.exe'" -ForegroundColor Gray
    if ($env:OS -notlike "*Windows*") {
        Write-Host "ℹ️  Non-Windows platform detected. Generated scripts and templates are ready for Windows CI/CD." -ForegroundColor Magenta
        exit 0
    }
}

# Step B: Perform requested Action
switch ($Action) {
    'CreateCertOnly' {
        $cert = New-CodeSignCertificate -CertSubject $Subject
        Export-PfxFile -Certificate $cert -Path $PfxPath -Pwd $Password
        if ($TrustLocally) { Install-TrustLocally -Path $PfxPath -Pwd $Password }
    }

    'TrustCertLocally' {
        if (-not (Test-Path $PfxPath)) { throw "PFX file not found at: $PfxPath" }
        Install-TrustLocally -Path $PfxPath -Pwd $Password
    }

    'SignOnly' {
        if (-not $TargetBinary) { throw "TargetBinary parameter is required for SignOnly." }
        if (-not (Test-Path $PfxPath)) { throw "PFX file not found at: $PfxPath" }
        Invoke-CodeSign -ToolPath $resolvedSigntool -FileToSign $TargetBinary -Pfx $PfxPath -Pwd $Password -PrimaryTs $TimestampUrl -FallbackTs $FallbackTimestampUrls
        if ($Verify) { Test-CodeSignature -ToolPath $resolvedSigntool -FileToCheck $TargetBinary }
    }

    'FullPipeline' {
        # 1. Ensure PFX certificate exists
        if (-not (Test-Path $PfxPath)) {
            Write-Host "📦 PFX not found at '$PfxPath'. Generating new certificate..." -ForegroundColor Cyan
            $cert = New-CodeSignCertificate -CertSubject $Subject
            Export-PfxFile -Certificate $cert -Path $PfxPath -Pwd $Password
            if ($TrustLocally) { Install-TrustLocally -Path $PfxPath -Pwd $Password }
        } else {
            Write-Host "✓ Existing PFX detected at: $PfxPath" -ForegroundColor Green
            if ($TrustLocally) { Install-TrustLocally -Path $PfxPath -Pwd $Password }
        }

        # 2. Sign binary if provided
        if ($TargetBinary -and (Test-Path $TargetBinary)) {
            Invoke-CodeSign -ToolPath $resolvedSigntool -FileToSign $TargetBinary -Pfx $PfxPath -Pwd $Password -PrimaryTs $TimestampUrl -FallbackTs $FallbackTimestampUrls
            if ($Verify) { Test-CodeSignature -ToolPath $resolvedSigntool -FileToCheck $TargetBinary }
        } elseif ($TargetBinary) {
            Write-Host "⚠️  Target binary specified but not found on filesystem: $TargetBinary" -ForegroundColor Yellow
        } else {
            Write-Host "ℹ️  No -TargetBinary passed. Certificate is ready in $PfxPath for CI/CD signing." -ForegroundColor Cyan
        }
    }
}

Write-Host "`n🎉 PowerShell code signing procedure completed." -ForegroundColor Cyan
