//! Clow Deck — app de bandeja (M3): embute o agente (lib `clowdeck-agent`) e
//! mostra o deck virtual numa janela. Menu: status, abrir deck, hooks, login, sair.

#![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "windows")]

use clowdeck_agent as agent;
use std::sync::OnceLock;
use tauri::menu::{CheckMenuItemBuilder, MenuBuilder, MenuItemBuilder, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{ActivationPolicy, AppHandle, Manager};

static SHARED: OnceLock<agent::app::Shared> = OnceLock::new();

fn port() -> u16 {
    SHARED.get().map(|s| s.cfg.port).unwrap_or(47831)
}

fn open_deck(h: &AppHandle) {
    if let Some(w) = h.get_webview_window("deck") {
        let _ = w.show();
        let _ = w.set_focus();
        return;
    }
    let url: tauri::Url = match format!("http://127.0.0.1:{}/", port()).parse() {
        Ok(u) => u,
        Err(_) => return,
    };
    if let Err(e) = tauri::WebviewWindowBuilder::new(h, "deck", tauri::WebviewUrl::External(url))
        .title("Clow Deck")
        .inner_size(430.0, 820.0)
        .build()
    {
        tracing::error!("janela do deck: {e}");
    }
}

fn main() {
    if let Some(b) = directories::BaseDirs::new() {
        let dir = b.home_dir().join("Library/Logs/clowdeck");
        let _ = std::fs::create_dir_all(&dir);
        let _ = agent::init_tracing_file(&dir.join("app.log"));
    }
    tracing::info!("Clow Deck app {} iniciando", env!("CARGO_PKG_VERSION"));

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|h, _argv, _cwd| open_deck(h)))
        .setup(|app| {
            // app de menu-bar: sem icone no Dock
            #[cfg(target_os = "macos")]
            app.set_activation_policy(ActivationPolicy::Accessory);
            // o app embute o agente — derruba o servico CLI antigo (mesma porta)
            agent::service::bootout_cli_quiet();

            // agente embutido; congelou -> restart() do proprio app
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                // o bootout do servico CLI e assincrono: a porta pode demorar alguns
                // segundos para ser liberada — tenta o bind por ate 15 s
                let mut bound = None;
                for tent in 0..30u32 {
                    let (cfg, cfg_path) = match agent::config::load_or_create() {
                        Ok(v) => v,
                        Err(e) => {
                            tracing::error!("config: {e:#}");
                            return;
                        }
                    };
                    match agent::bind(cfg, cfg_path, false, true).await {
                        Ok(v) => {
                            bound = Some(v);
                            break;
                        }
                        Err(e) => {
                            if tent == 0 {
                                tracing::warn!("bind: {e:#} — aguardando a porta liberar");
                            }
                            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                        }
                    }
                }
                let Some((st, listener)) = bound else {
                    tracing::error!("agente nao subiu: porta ocupada apos 15 s");
                    return;
                };
                let _ = SHARED.set(st.clone());
                let h = handle.clone();
                let on_freeze: agent::FreezeAction = Box::new(move || h.restart());
                if let Err(e) = agent::serve(st, listener, Some(on_freeze)).await {
                    tracing::error!("agente: {e:#}");
                }
            });

            // menu da bandeja
            let status = MenuItemBuilder::with_id("status", "iniciando…").enabled(false).build(app)?;
            let hooks_st = MenuItemBuilder::with_id("hooks_st", "hooks: verificando…").enabled(false).build(app)?;
            let open = MenuItemBuilder::with_id("open", "Abrir deck virtual").build(app)?;
            let hooks_act = MenuItemBuilder::with_id("hooks", "Instalar hooks do Claude Code").build(app)?;
            let login = CheckMenuItemBuilder::with_id("login", "Iniciar no login")
                .checked(agent::service::app_login_installed())
                .build(app)?;
            let quit = MenuItemBuilder::with_id("quit", "Sair").build(app)?;
            let menu = MenuBuilder::new(app)
                .item(&status)
                .item(&hooks_st)
                .item(&PredefinedMenuItem::separator(app)?)
                .item(&open)
                .item(&hooks_act)
                .item(&login)
                .item(&PredefinedMenuItem::separator(app)?)
                .item(&quit)
                .build()?;

            let login_c = login.clone();
            let tray = TrayIconBuilder::with_id("clow")
                .icon(tauri::image::Image::from_bytes(include_bytes!("../icons/tray-off.png"))?)
                .icon_as_template(false)
                .menu(&menu)
                .on_menu_event(move |h, ev| match ev.id().as_ref() {
                    "open" => open_deck(h),
                    "hooks" => match agent::hooks::install(&agent::hooks::default_settings_path(), port()) {
                        Ok(r) => tracing::info!("hooks instalados ({} eventos)", r.installed.len()),
                        Err(e) => tracing::error!("hooks: {e:#}"),
                    },
                    "login" => {
                        // o clique ja alternou o check; sincroniza o launchd com ele
                        let on = login_c.is_checked().unwrap_or(false);
                        let r = if on {
                            agent::service::app_login_install()
                        } else {
                            agent::service::app_login_uninstall()
                        };
                        if let Err(e) = r {
                            tracing::error!("login item: {e:#}");
                            let _ = login_c.set_checked(!on);
                        }
                    }
                    "quit" => h.exit(0),
                    _ => {}
                })
                .build(app)?;

            // status a cada 5 s (+ cor do icone da bandeja: coral = placa conectada)
            let tray_c = tray.clone();
            tauri::async_runtime::spawn(async move {
                let mut last_conn: Option<bool> = None;
                loop {
                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                    let Some(st) = SHARED.get() else { continue };
                    let n = {
                        let m = st.model();
                        (0..agent::protocol::SESSION_CELLS).filter(|&i| m.cell(i).is_some()).count()
                    };
                    let connected = st.ble_status().connected;
                    if last_conn != Some(connected) {
                        let bytes: &[u8] = if connected {
                            include_bytes!("../icons/tray-on.png")
                        } else {
                            include_bytes!("../icons/tray-off.png")
                        };
                        if let Ok(img) = tauri::image::Image::from_bytes(bytes) {
                            let _ = tray_c.set_icon(Some(img));
                        }
                        last_conn = Some(connected);
                    }
                    let _ = status.set_text(format!(
                        "{n} sessao(oes) · deck {}",
                        if connected { "conectado" } else { "procurando" }
                    ));
                    match agent::hooks::status(&agent::hooks::default_settings_path()) {
                        Ok(s) if !s.installed_events.is_empty() && s.missing_events.is_empty() => {
                            let _ = hooks_st.set_text("hooks: instalados");
                            let _ = hooks_act.set_enabled(false);
                        }
                        _ => {
                            let _ = hooks_st.set_text("hooks: nao instalados");
                            let _ = hooks_act.set_enabled(true);
                        }
                    }
                }
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("erro ao iniciar o Clow Deck");
}
