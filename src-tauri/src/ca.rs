use log::info;

/// Check if the CA certificate is installed in the system trust store
#[tauri::command]
pub(crate) fn is_ca_installed() -> bool {
    #[cfg(target_os = "macos")]
    {
        let output = std::process::Command::new("security")
            .args(["find-certificate", "-c", "KanColle Browser CA"])
            .output();
        match output {
            Ok(o) => o.status.success(),
            Err(_) => false,
        }
    }
    #[cfg(target_os = "windows")]
    {
        let output = std::process::Command::new("certutil")
            .args(["-verifystore", "Root", "KanColle Browser CA"])
            .output();
        match output {
            Ok(o) => o.status.success(),
            Err(_) => false,
        }
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        false
    }
}

/// Install the CA certificate into the system trust store.
#[tauri::command]
pub(crate) fn install_ca_cert() -> Result<(), String> {
    let pem_path = crate::proxy::ca_pem_path();

    if !pem_path.exists() {
        return Err("CA certificate file not found. Proxy may not have started yet.".to_string());
    }

    let pem_str = pem_path.to_str().unwrap();
    info!("Installing CA certificate from: {}", pem_path.display());

    #[cfg(target_os = "macos")]
    {
        let keychain = format!(
            "{}/Library/Keychains/login.keychain-db",
            std::env::var("HOME").unwrap_or_default()
        );

        // Step 1: Import certificate to login keychain
        let import_output = std::process::Command::new("security")
            .args(["import", pem_str, "-k", &keychain, "-t", "cert"])
            .output()
            .map_err(|e| format!("Failed to run security import: {}", e))?;

        if !import_output.status.success() {
            let stderr = String::from_utf8_lossy(&import_output.stderr);
            if !stderr.contains("already exists") {
                return Err(format!("Failed to import certificate: {}", stderr));
            }
            info!("CA certificate already in keychain, updating trust...");
        } else {
            info!("CA certificate imported to keychain");
        }

        // Step 2: Set trust as root CA (triggers macOS password dialog)
        let trust_output = std::process::Command::new("security")
            .args([
                "add-trusted-cert",
                "-d",
                "-r",
                "trustRoot",
                "-k",
                &keychain,
                pem_str,
            ])
            .output()
            .map_err(|e| format!("Failed to run security add-trusted-cert: {}", e))?;

        if trust_output.status.success() {
            info!("CA certificate trusted for SSL successfully");
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&trust_output.stderr);
            Err(format!("Failed to set certificate trust: {}", stderr))
        }
    }

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;

        // Use PowerShell Start-Process -Verb RunAs to trigger UAC elevation dialog
        // This allows certificate installation without running the app as administrator
        let escaped_path = pem_str.replace('\'', "''");
        let script = format!(
            "try {{ $p = Start-Process -FilePath 'certutil.exe' -ArgumentList '-addstore','Root','\"{}\"' -Verb RunAs -Wait -PassThru; exit $p.ExitCode }} catch {{ Write-Error $_.Exception.Message; exit 1 }}",
            escaped_path
        );

        let output = std::process::Command::new("powershell.exe")
            .args(["-NoProfile", "-Command", &script])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .map_err(|e| format!("Failed to run certutil: {}", e))?;

        if output.status.success() {
            info!("CA certificate installed to Windows trust store");
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("canceled") || stderr.contains("cancelled") {
                Err("Certificate installation was cancelled by user.".to_string())
            } else {
                Err(format!("Failed to install certificate: {}", stderr))
            }
        }
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        Err("Certificate installation not supported on this platform".to_string())
    }
}
