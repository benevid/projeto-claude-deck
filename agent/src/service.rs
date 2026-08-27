//! `service install|uninstall|status`: roda o agente como LaunchAgent do usuario
//! (macOS). KeepAlive reinicia em 5 s se o processo cair (watchdog, panico, etc.) e
//! RunAtLoad sobe no login. Em outros SOs e um stub (M4).

#[cfg(not(target_os = "macos"))]
use anyhow::Result;

pub const LABEL: &str = "my.autom.clowdeck";

#[cfg(target_os = "macos")]
mod imp {
    use super::LABEL;
    use anyhow::{anyhow, Context, Result};
    use std::path::PathBuf;
    use std::process::Command;

    fn home() -> Result<PathBuf> {
        directories::BaseDirs::new().map(|b| b.home_dir().to_path_buf()).context("sem HOME")
    }
    fn plist_path() -> Result<PathBuf> {
        Ok(home()?.join("Library/LaunchAgents").join(format!("{LABEL}.plist")))
    }
    fn uid() -> Result<String> {
        let out = Command::new("id").arg("-u").output().context("id -u")?;
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
    }
    fn launchctl(args: &[&str]) -> Result<(bool, String)> {
        let out = Command::new("launchctl").args(args).output().context("launchctl")?;
        let txt = format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
        Ok((out.status.success(), txt))
    }

    /// Escolhe a identidade de assinatura: a configurada, senao a primeira
    /// "Developer ID Application", senao a primeira "Apple Development".
    fn pick_identity(cfg: Option<&str>) -> Option<String> {
        if let Some(c) = cfg {
            if !c.trim().is_empty() {
                return Some(c.trim().to_string());
            }
        }
        let out = Command::new("security").args(["find-identity", "-v", "-p", "codesigning"]).output().ok()?;
        let txt = String::from_utf8_lossy(&out.stdout);
        let ids: Vec<(String, String)> = txt
            .lines()
            .filter_map(|l| {
                let l = l.trim();
                let hash = l.split(')').nth(1)?.trim().split(' ').next()?.to_string();
                let name = l.split('"').nth(1)?.to_string();
                if hash.len() == 40 { Some((hash, name)) } else { None }
            })
            .collect();
        ids.iter().find(|(_, n)| n.starts_with("Developer ID Application"))
            .or_else(|| ids.iter().find(|(_, n)| n.starts_with("Apple Development")))
            .map(|(h, n)| { println!("assinando com: {n}"); h.clone() })
    }

    /// Assina o binario com identificador FIXO: o TCC guarda as permissoes pelo
    /// "designated requirement"; com assinatura ad-hoc (padrao do linker no Apple
    /// Silicon) isso e o cdhash, que muda a cada build — Bluetooth e Acessibilidade
    /// caem toda vez que se recompila (visto na bancada).
    fn sign(exe: &std::path::Path, identity: &str) -> Result<()> {
        let out = Command::new("codesign")
            .args(["--force", "--sign", identity, "--identifier", "my.autom.clowdeck-agent", "--timestamp=none"])
            .arg(exe)
            .output()
            .context("codesign")?;
        if !out.status.success() {
            return Err(anyhow!("codesign falhou: {}", String::from_utf8_lossy(&out.stderr).trim()));
        }
        Ok(())
    }

    pub fn install(port: u16, sign_identity: Option<&str>) -> Result<()> {
        let exe = std::env::current_exe()?.canonicalize()?;
        let log_dir = home()?.join("Library/Logs/clowdeck");
        std::fs::create_dir_all(&log_dir)?;
        let log = log_dir.join("agent.log");
        let plist = plist_path()?;
        std::fs::create_dir_all(plist.parent().unwrap())?;
        let xml = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key><string>{LABEL}</string>
  <key>ProgramArguments</key>
  <array>
    <string>{exe}</string>
    <string>run</string>
    <string>--port</string>
    <string>{port}</string>
  </array>
  <key>RunAtLoad</key><true/>
  <key>KeepAlive</key><true/>
  <key>ThrottleInterval</key><integer>5</integer>
  <key>ProcessType</key><string>Interactive</string>
  <key>StandardOutPath</key><string>{log}</string>
  <key>StandardErrorPath</key><string>{log}</string>
  <key>EnvironmentVariables</key>
  <dict>
    <key>PATH</key>
    <string>/usr/local/bin:/opt/homebrew/bin:/usr/bin:/bin:/usr/sbin:/sbin</string>
  </dict>
</dict>
</plist>
"#,
            exe = exe.display(),
            log = log.display()
        );
        std::fs::write(&plist, xml).with_context(|| format!("escrevendo {}", plist.display()))?;
        let domain = format!("gui/{}", uid()?);
        let _ = launchctl(&["bootout", &format!("{domain}/{LABEL}")]); // pode nao existir
        match pick_identity(sign_identity) {
            Some(id) => sign(&exe, &id)?,
            None => println!("AVISO: nenhuma identidade de assinatura (security find-identity) — assinatura ad-hoc; as permissoes do macOS vao cair a cada recompilacao"),
        }
        // logo apos o bootout o launchd ainda esta derrubando o job antigo e o bootstrap
        // responde "5: Input/output error" — re-tentar por alguns segundos resolve.
        let mut last = String::new();
        let mut ok = false;
        for _ in 0..8 {
            let (o, txt) = launchctl(&["bootstrap", &domain, plist.to_str().unwrap()])?;
            if o {
                ok = true;
                break;
            }
            last = txt;
            std::thread::sleep(std::time::Duration::from_millis(800));
        }
        if !ok {
            return Err(anyhow!("launchctl bootstrap falhou: {}", last.trim()));
        }
        println!("servico {LABEL} instalado e iniciado");
        println!("plist: {}", plist.display());
        println!("log:   {}", log.display());
        println!("binario: {} (recompilou? rode `service install` de novo p/ apontar o novo)", exe.display());
        println!("macOS vai pedir Bluetooth (e Acessibilidade/Automacao ao focar) para `clowdeck-agent` na primeira vez");
        Ok(())
    }

    pub fn uninstall() -> Result<()> {
        let domain = format!("gui/{}", uid()?);
        let (ok, txt) = launchctl(&["bootout", &format!("{domain}/{LABEL}")])?;
        if !ok && !txt.contains("No such process") && !txt.contains("Could not find") {
            eprintln!("launchctl bootout: {}", txt.trim());
        }
        let plist = plist_path()?;
        if plist.exists() {
            std::fs::remove_file(&plist)?;
        }
        println!("servico {LABEL} removido");
        Ok(())
    }

    pub fn status() -> Result<()> {
        let domain = format!("gui/{}", uid()?);
        let (ok, txt) = launchctl(&["print", &format!("{domain}/{LABEL}")])?;
        if !ok {
            println!("servico {LABEL}: nao carregado (plist {}existe)", if plist_path()?.exists() { "" } else { "nao " });
            return Ok(());
        }
        let state = txt.lines().find(|l| l.trim_start().starts_with("state =")).map(|l| l.trim().to_string());
        let pid = txt.lines().find(|l| l.trim_start().starts_with("pid =")).map(|l| l.trim().to_string());
        println!("servico {LABEL}: {} {}", state.unwrap_or_else(|| "?".into()), pid.unwrap_or_default());
        Ok(())
    }

    // ---- App de bandeja (M3): takeover do servico CLI + login item do proprio app ----
    pub const APP_LABEL: &str = "my.autom.clowdeck.app";

    fn app_plist_path() -> Result<PathBuf> {
        Ok(home()?.join("Library/LaunchAgents").join(format!("{APP_LABEL}.plist")))
    }

    /// O app embute o agente; ele e o servico CLI nao podem coexistir (mesma porta).
    /// Derruba o servico silenciosamente e remove o plist p/ nao voltar no login.
    pub fn bootout_cli_quiet() {
        if let (Ok(u), Ok(p)) = (uid(), plist_path()) {
            if p.exists() {
                let _ = launchctl(&["bootout", &format!("gui/{u}/{LABEL}")]);
                let _ = std::fs::remove_file(&p);
                tracing::info!("servico CLI {LABEL} desativado (o app assume o agente)");
            }
        }
    }

    pub fn app_login_installed() -> bool {
        app_plist_path().map(|p| p.exists()).unwrap_or(false)
    }

    /// RunAtLoad + KeepAlive(SuccessfulExit=false): abre no login e volta sozinho se
    /// crashar (a partir do proximo login), mas respeita o "Sair" (exit 0).
    pub fn app_login_install() -> Result<()> {
        let exe = std::env::current_exe()?.canonicalize()?;
        let log_dir = home()?.join("Library/Logs/clowdeck");
        std::fs::create_dir_all(&log_dir)?;
        let log = log_dir.join("app.log");
        let plist = app_plist_path()?;
        std::fs::create_dir_all(plist.parent().unwrap())?;
        let xml = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key><string>{APP_LABEL}</string>
  <key>ProgramArguments</key>
  <array>
    <string>{exe}</string>
  </array>
  <key>RunAtLoad</key><true/>
  <key>KeepAlive</key>
  <dict>
    <key>SuccessfulExit</key><false/>
  </dict>
  <key>ThrottleInterval</key><integer>5</integer>
  <key>ProcessType</key><string>Interactive</string>
  <key>StandardOutPath</key><string>{log}</string>
  <key>StandardErrorPath</key><string>{log}</string>
</dict>
</plist>
"#,
            exe = exe.display(),
            log = log.display()
        );
        std::fs::write(&plist, xml).with_context(|| format!("escrevendo {}", plist.display()))?;
        Ok(())
    }

    pub fn app_login_uninstall() -> Result<()> {
        if let Ok(u) = uid() {
            let _ = launchctl(&["bootout", &format!("gui/{u}/{APP_LABEL}")]);
        }
        let plist = app_plist_path()?;
        if plist.exists() {
            std::fs::remove_file(&plist)?;
        }
        Ok(())
    }
}

#[cfg(target_os = "macos")]
pub use imp::{app_login_install, app_login_installed, app_login_uninstall, bootout_cli_quiet, install, status, uninstall};

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub fn bootout_cli_quiet() {}
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub fn app_login_installed() -> bool { false }
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub fn app_login_install() -> Result<()> { anyhow::bail!("login item: so macOS") }
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub fn app_login_uninstall() -> Result<()> { anyhow::bail!("login item: so macOS") }

// ---- Windows (M4): inicio no login pela chave Run do usuario ----
#[cfg(target_os = "windows")]
mod win {
    use anyhow::{Context, Result};
    use std::process::Command;

    pub const RUN_KEY: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";
    pub const VALUE: &str = "ClowDeck";

    fn reg(args: &[&str]) -> Result<(bool, String)> {
        let out = Command::new("reg").args(args).output().context("reg.exe")?;
        let txt = format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
        Ok((out.status.success(), txt))
    }

    pub fn install(port: u16) -> Result<()> {
        let exe = std::env::current_exe()?;
        let cmd = format!("\"{}\" run --port {port}", exe.display());
        let (ok, txt) = reg(&["add", RUN_KEY, "/v", VALUE, "/t", "REG_SZ", "/d", &cmd, "/f"])?;
        if !ok {
            anyhow::bail!("reg add falhou: {}", txt.trim());
        }
        println!("inicio no login registrado em {RUN_KEY}\\{VALUE}");
        println!("comando: {cmd}");
        Ok(())
    }

    pub fn uninstall() -> Result<()> {
        let (ok, txt) = reg(&["delete", RUN_KEY, "/v", VALUE, "/f"])?;
        if !ok && !txt.contains("nao foi possivel") && !txt.contains("unable to find") {
            eprintln!("reg delete: {}", txt.trim());
        }
        println!("inicio no login removido");
        Ok(())
    }

    pub fn status() -> Result<()> {
        let (ok, txt) = reg(&["query", RUN_KEY, "/v", VALUE])?;
        if ok {
            println!("inicio no login: ATIVO\n{}", txt.trim());
        } else {
            println!("inicio no login: nao configurado");
        }
        Ok(())
    }
}

#[cfg(target_os = "windows")]
pub fn install(port: u16, _sign_identity: Option<&str>) -> Result<()> {
    win::install(port)
}
#[cfg(target_os = "windows")]
pub fn uninstall() -> Result<()> {
    win::uninstall()
}
#[cfg(target_os = "windows")]
pub fn status() -> Result<()> {
    win::status()
}
#[cfg(target_os = "windows")]
pub fn bootout_cli_quiet() {
    let _ = std::process::Command::new("reg")
        .args(["delete", win::RUN_KEY, "/v", win::VALUE, "/f"])
        .output();
}
#[cfg(target_os = "windows")]
pub fn app_login_installed() -> bool {
    std::process::Command::new("reg")
        .args(["query", win::RUN_KEY, "/v", "ClowDeckApp"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}
#[cfg(target_os = "windows")]
pub fn app_login_install() -> Result<()> {
    let exe = std::env::current_exe()?;
    let out = std::process::Command::new("reg")
        .args(["add", win::RUN_KEY, "/v", "ClowDeckApp", "/t", "REG_SZ", "/d",
               &format!("\"{}\"", exe.display()), "/f"])
        .output()?;
    if !out.status.success() {
        anyhow::bail!("reg add (app) falhou");
    }
    Ok(())
}
#[cfg(target_os = "windows")]
pub fn app_login_uninstall() -> Result<()> {
    let _ = std::process::Command::new("reg")
        .args(["delete", win::RUN_KEY, "/v", "ClowDeckApp", "/f"])
        .output();
    Ok(())
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub fn install(_port: u16, _sign_identity: Option<&str>) -> Result<()> {
    anyhow::bail!("service: so macOS e Windows")
}
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub fn uninstall() -> Result<()> {
    anyhow::bail!("service: so macOS e Windows")
}
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub fn status() -> Result<()> {
    anyhow::bail!("service: so macOS e Windows")
}
