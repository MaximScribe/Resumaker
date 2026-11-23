# Installs Resumaker on Windows.
#
#   irm https://raw.githubusercontent.com/MaximScribe/Resumaker/main/install.ps1 | iex
#
# Downloads the latest release, verifies its SHA-256 checksum, puts resumaker.exe and
# resumake.exe in %LOCALAPPDATA%\Programs\Resumaker and adds that folder to your user PATH.
# Run it again to update.
#
# Optional environment variables:
#   RESUMAKER_VERSION         a release tag such as v1.0.1 (default: the latest release)
#   RESUMAKER_INSTALL_DIR     where the binaries go
#   RESUMAKER_NO_MODIFY_PATH  set to 1 to leave PATH alone

$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'   # the progress bar makes downloads very slow in PowerShell 5.1
[Net.ServicePointManager]::SecurityProtocol = [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12

# Everything runs inside a script block so that nothing leaks into the caller's session
# when the script is piped into `iex`, and errors are thrown instead of closing the window.
& {
    $repo = 'MaximScribe/Resumaker'
    $target = 'x86_64-pc-windows-msvc'   # also runs on Windows on ARM through x64 emulation

    $version = $env:RESUMAKER_VERSION
    if (-not $version) {
        $version = (Invoke-RestMethod "https://api.github.com/repos/$repo/releases/latest").tag_name
    }
    $dir = $env:RESUMAKER_INSTALL_DIR
    if (-not $dir) { $dir = Join-Path $env:LOCALAPPDATA 'Programs\Resumaker' }

    $name = "resumaker-$version-$target"
    $base = "https://github.com/$repo/releases/download/$version"
    $tmp = Join-Path ([IO.Path]::GetTempPath()) ("resumaker-install-" + [Guid]::NewGuid())
    New-Item -ItemType Directory $tmp | Out-Null

    try {
        Write-Host "Downloading Resumaker $version ..."
        $zip = Join-Path $tmp "$name.zip"
        Invoke-WebRequest "$base/$name.zip" -OutFile $zip -UseBasicParsing
        Invoke-WebRequest "$base/$name.zip.sha256" -OutFile "$zip.sha256" -UseBasicParsing

        $expected = ((Get-Content "$zip.sha256" -Raw).Trim() -split '\s+')[0].ToLower()
        $actual = (Get-FileHash $zip -Algorithm SHA256).Hash.ToLower()
        if ($expected -ne $actual) {
            throw "Checksum mismatch for $name.zip (expected $expected, got $actual). Nothing was installed."
        }

        Expand-Archive $zip -DestinationPath $tmp
        New-Item -ItemType Directory -Force $dir | Out-Null
        foreach ($exe in 'resumaker.exe', 'resumake.exe') {
            Copy-Item (Join-Path $tmp "$name\$exe") $dir -Force
        }
        Write-Host "Installed resumaker and resumake to $dir"
    }
    finally {
        Remove-Item -Recurse -Force $tmp -ErrorAction SilentlyContinue
    }

    if ($env:RESUMAKER_NO_MODIFY_PATH -eq '1') { return }

    # Read the user PATH straight from the registry without expanding %VARIABLES%, so
    # entries like %USERPROFILE%\bin stay intact when it is written back.
    $key = [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey('Environment', $true)
    $userPath = [string]$key.GetValue('Path', '', 'DoNotExpandEnvironmentNames')
    $entries = $userPath -split ';' | Where-Object { $_ }
    if ($entries | Where-Object { $_.TrimEnd('\') -ieq $dir.TrimEnd('\') }) {
        Write-Host "$dir is already on your PATH."
    }
    else {
        $key.SetValue('Path', (($entries + $dir) -join ';'), [Microsoft.Win32.RegistryValueKind]::ExpandString)
        # Setting any user variable through .NET tells Explorer that the environment changed,
        # so terminals opened from now on see the new PATH.
        [Environment]::SetEnvironmentVariable('RESUMAKER_INSTALL_TMP', '1', 'User')
        [Environment]::SetEnvironmentVariable('RESUMAKER_INSTALL_TMP', $null, 'User')
        Write-Host "Added $dir to your PATH."
    }
    $key.Close()
    if (-not (($env:Path -split ';') -contains $dir)) { $env:Path = "$env:Path;$dir" }

    Write-Host ""
    Write-Host "Done. Run 'resumaker' to set up, then 'resumake' to make a resume."
    Write-Host "Other terminals that were already open (and VS Code) need a restart to find the commands."
}
