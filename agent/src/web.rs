//! Deck virtual + API local: `GET /` (grade 4x3 no navegador), `GET /state`,
//! `POST /event` (mesma semantica dos EVENTs BLE). Mesmo servidor dos hooks.

use crate::app::Shared;
use crate::dispatch;
use crate::protocol::{Action, Event, EventKind, Mode, State, PROTO_VERSION};
use axum::extract::State as AxState;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse};
use axum::Json;
use serde_json::{json, Value};

fn mode_of(v: u8) -> Mode {
    match v {
        1 => Mode::Default,
        2 => Mode::AcceptEdits,
        3 => Mode::Plan,
        4 => Mode::Bypass,
        5 => Mode::DontAsk,
        _ => Mode::Unknown,
    }
}

fn state_of(v: u8) -> State {
    match v {
        1 => State::Unknown,
        2 => State::Working,
        3 => State::Attention,
        4 => State::Done,
        5 => State::Idle,
        6 => State::Error,
        7 => State::Dead,
        _ => State::Empty,
    }
}

pub fn state_json(st: &Shared) -> Value {
    let m = st.model();
    let view = m.view(false, false);
    let (open, attention, done) = m.counts();
    let cells: Vec<Value> = view
        .cells
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let s = m.cell(i);
            json!({
                "cell": i,
                "sid": c.sid,
                "state": c.state,
                "state_name": state_of(c.state).name(),
                "mode": c.mode,
                "mode_name": mode_of(c.mode).short(),
                "active": c.active,
                "no_hooks": c.no_hooks,
                "codex": c.codex,
                "opencode": c.opencode,
                "age_s": c.age_s,
                "label": c.label,
                "pid": s.and_then(|s| s.pid),
                "tty": s.and_then(|s| s.tty.clone()),
                "terminal": s.and_then(|s| s.terminal_app.map(|t| t.name())),
                "cwd": s.map(|s| s.cwd.clone()),
                "session_id": s.and_then(|s| s.session_id.clone()),
                "last_event": s.map(|s| s.last_event.clone()),
            })
        })
        .collect();
    let ble = st.ble_status().clone();
    json!({
        "version": env!("CARGO_PKG_VERSION"),
        "proto": PROTO_VERSION,
        "uptime_s": st.started.elapsed().as_secs(),
        "hooks_received": st.hooks_count(),
        "dry_run": st.dry_run,
        "port": st.cfg.port,
        "config_path": st.config_path.display().to_string(),
        "ble": ble,
        "active": view.active,
        "counts": { "open": open, "attention": attention, "done": done, "waiting": m.overflow_len() },
        "cells": cells,
        "commands": st.cfg.commands,
        "deck": { "brightness": st.cfg.deck.brightness, "lang": st.cfg.deck.lang },
        "layout": {
            "cols": ble.info.as_ref().map(|i| i.cols).unwrap_or(3),
            "rows": ble.info.as_ref().map(|i| i.rows).unwrap_or(4),
            "session_cells": ble.info.as_ref().map(|i| i.session_cells).unwrap_or(6),
            "capacity": m.capacity(),
        },
        "last_hook": st.last_hook.lock().unwrap_or_else(|e| e.into_inner()).clone(),
        "recent_hooks": st.recent_hooks.lock().unwrap_or_else(|e| e.into_inner()).iter().cloned().collect::<Vec<_>>(),
    })
}

pub async fn get_state(AxState(st): AxState<Shared>) -> Json<Value> {
    Json(state_json(&st))
}

pub async fn health() -> Json<Value> {
    Json(json!({"ok": true, "proto": PROTO_VERSION}))
}

#[derive(serde::Deserialize)]
pub struct EventBody {
    kind: Value,
    #[serde(default)]
    cell: Option<u8>,
    #[serde(default)]
    arg: Option<Value>,
}

fn parse_event(b: &EventBody) -> Result<Event, String> {
    let kind = match &b.kind {
        Value::Number(n) => n.as_u64().and_then(|v| EventKind::from_u8(v as u8)),
        Value::String(s) => EventKind::from_name(s),
        _ => None,
    }
    .ok_or_else(|| format!("kind invalido: {}", b.kind))?;
    let arg = match &b.arg {
        None | Some(Value::Null) => 0,
        Some(Value::Number(n)) => n.as_u64().ok_or("arg invalido")? as u8,
        Some(Value::String(s)) => {
            if kind == EventKind::Action {
                Action::from_name(s).map(|a| a.to_u8()).ok_or_else(|| format!("acao desconhecida: {s}"))?
            } else {
                s.parse::<u8>().map_err(|_| "arg invalido".to_string())?
            }
        }
        Some(v) => return Err(format!("arg invalido: {v}")),
    };
    let cell = b.cell.unwrap_or(if kind == EventKind::Action { 0xFF } else { 0 });
    Ok(Event { kind, cell, arg })
}

pub async fn post_event(AxState(st): AxState<Shared>, Json(body): Json<EventBody>) -> impl IntoResponse {
    let ev = match parse_event(&body) {
        Ok(e) => e,
        Err(e) => return (StatusCode::BAD_REQUEST, Json(json!({"ok": false, "error": e}))),
    };
    match dispatch::handle_event(&st, ev, "web").await {
        Ok(r) => (StatusCode::OK, Json(json!({"ok": true, "result": r, "event": ev.encode()}))),
        Err(e) => (StatusCode::OK, Json(json!({"ok": false, "error": format!("{e:#}"), "event": ev.encode()}))),
    }
}

pub async fn index() -> Html<&'static str> {
    Html(INDEX_HTML)
}

const INDEX_HTML: &str = r##"<!doctype html>
<html lang="pt-BR"><head><meta charset="utf-8"><title>Clow Deck — virtual</title>
<meta name="viewport" content="width=device-width,initial-scale=1">
<link href="https://fonts.googleapis.com/css2?family=Montserrat:wght@500;600;700&display=swap" rel="stylesheet">
<link rel="icon" href="data:image/svg+xml,%3Csvg%20xmlns%3D%22http%3A%2F%2Fwww.w3.org%2F2000%2Fsvg%22%20viewBox%3D%220%200%2032%2032%22%3E%3Crect%20width%3D%2232%22%20height%3D%2232%22%20rx%3D%227%22%20fill%3D%22%23141413%22%2F%3E%3Cpath%20d%3D%22M8%206h3v1h-3zM21%206h3v1h-3zM8%207h3v1h-3zM21%207h3v1h-3zM8%208h5v1h-5zM19%208h5v1h-5zM10%209h3v1h-3zM19%209h3v1h-3zM8%2010h16v1h-16zM8%2011h16v1h-16zM6%2012h4v1h-4zM13%2012h6v1h-6zM22%2012h4v1h-4zM6%2013h4v1h-4zM13%2013h6v1h-6zM22%2013h4v1h-4zM6%2014h4v1h-4zM13%2014h6v1h-6zM22%2014h4v1h-4zM6%2015h20v1h-20zM4%2016h24v1h-24zM4%2017h24v1h-24zM4%2018h24v1h-24zM4%2019h3v1h-3zM8%2019h16v1h-16zM25%2019h3v1h-3zM4%2020h3v1h-3zM8%2020h16v1h-16zM25%2020h3v1h-3zM4%2021h3v1h-3zM8%2021h3v1h-3zM21%2021h3v1h-3zM25%2021h3v1h-3zM4%2022h3v1h-3zM8%2022h7v1h-7zM17%2022h7v1h-7zM25%2022h3v1h-3zM10%2023h5v1h-5zM17%2023h5v1h-5zM10%2024h5v1h-5zM17%2024h5v1h-5z%22%20fill%3D%22%23D97757%22%2F%3E%3C%2Fsvg%3E">
<style>
:root{--bg0:#0C0D0C;--bg1:#141513;--cell:#131412;--cline:#3A3B39;--cdim:#242523;--press:#1E1F1D;--text:#E3E2DF;--muted:#B0AEA5;--faint:#6B6960;--accent:#D97757;--accenthi:#E2957F;--work:#A3E635;--attn:#F0B35B;--ok:#788C5D;--bad:#C8524A;--warn:#F0B35B}
*{box-sizing:border-box}
body{margin:0;background:linear-gradient(#0C0D0C,#141513) fixed;color:var(--text);font:14px/1.4 Montserrat,-apple-system,sans-serif}
body::before{content:'';position:fixed;inset:0;pointer-events:none;background-image:radial-gradient(circle at 12% 18%,rgba(217,119,87,.14) 0 1px,transparent 2px),radial-gradient(circle at 78% 9%,rgba(217,119,87,.10) 0 1px,transparent 2px),radial-gradient(circle at 33% 64%,rgba(217,119,87,.12) 0 1.4px,transparent 2.4px),radial-gradient(circle at 89% 47%,rgba(217,119,87,.08) 0 1px,transparent 2px),radial-gradient(circle at 55% 88%,rgba(217,119,87,.12) 0 1px,transparent 2px),radial-gradient(circle at 8% 84%,rgba(217,119,87,.09) 0 1.2px,transparent 2.2px)}
header{display:flex;gap:16px;align-items:center;padding:10px 18px;border-bottom:1px solid #1F201E}
header b{color:var(--accent);letter-spacing:1px}header span{color:var(--muted)}header .dot{width:9px;height:9px;border-radius:50%;background:var(--faint);display:inline-block;margin-right:6px}
header .dot.on{background:var(--ok)}
.appicon{width:34px;height:34px;flex:none;image-rendering:pixelated}.appicon .lid{opacity:0;animation:clowblink 3.2s steps(1,end) infinite}@keyframes clowblink{0%,91%{opacity:0}92%,96%{opacity:1}97%,100%{opacity:0}}
#deck{width:min(92vw,440px);aspect-ratio:320/480;margin:18px auto;display:grid;grid-template-columns:repeat(3,1fr);grid-template-rows:repeat(4,1fr);gap:6px;padding:6px;background:transparent;border-radius:14px}
.cell{position:relative;border-radius:16px;background:var(--cell);border:2px solid var(--cdim);cursor:pointer;user-select:none;overflow:hidden;transition:background .1s}
.cell:active{background:var(--press)}
.cell .masc{position:absolute;left:50%;top:42%;width:52px;height:52px;transform:translate(-50%,-50%);background:var(--faint);opacity:.55;-webkit-mask:var(--m) center/contain no-repeat;mask:var(--m) center/contain no-repeat}
.cell .mclaude{--m:url(data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAACgAAAAoCAYAAACM/rhtAAACu0lEQVR4nO2YO28TQRDHf+c7h5NDeMggCpAoKAAh0QToaXhUSKBUfJN8hzR8hQgpFRIViIaSgKCJFCkULpAgEuItRxYPn5diZ5z1+ny3+Ezswn/pdJ7defxvdnZ2k8gYwyyjNm0CZZgTrIo5waqYeYJJyXx0ICxgZK8rIzj1JplHMMISS4HHwEmRJ51N9fkJuAP8dGIXElTUgGvAsQkT8/Gdgr1QtsR7wBGgJ04y7BfWipyOQE+eCIgdn3tFRmUEXSLjkPJ9FfnPRRlBhdbFG+AdcAk4T1htqs5bYBs4CywTuAFDM9KT9xpwD9gQOQuwVZ0NsV3zfBbCzWCEJaw7Kc7RXwIWgEaIcw8NsV3KmYvl0dg9eQ8QNAxmpM3wMrSB30BnDIIdsW174yZnrI/EYd0ErjDYB1PR0zq7CnwFLnjjRVCdi8BN8eGOpwz3wdfAFyCKjDExNnO3gCcBAQ8Ct4GnQOwucRdLVPsTDNeh1obW6r9Aa0tr3YWWlsbu6oTfZiLnyUOVPqgNetScG7+PxPtdtRmPi5r37vOKjDFamMeBy47RIeAhcILBZZ8U1Odn4D7wy5nbAr4hm6TIyXvg9H8m+AE4M0qpqFE3xEmX/brwLwt+78yDNmD/smDEdw/bvDsENmpXqenp+EUeEX6e+/WtvpoSy43dR5HzP8AD4ChwFzgFPAd2sPfEZaAFPJPA/tmqYzeAc9iLxitsk78OfAQeAT8kVj6MMSHPprFYEXlV5PUA23XRXRV5ReTNkNhly1Nnv+Yy4LBkvSHygsh6GrmIHZ1MbBLxkYlPPWpHZrCMYFcILkrARMbqIqeOjk9Qx1LRrYuuftAigxtwLIKKl9ha2RW5BbzA9isl40PHtrB/eLVE3hXbnZDAZX1w6gjNoLsM/jW/7Aur2M5+Bmf+fzNzglUxJ1gVM0/wL6JcK3Wr3mcIAAAAAElFTkSuQmCC)}
.cell .mcodex{--m:url(data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAACgAAAAoCAYAAACM/rhtAAAEtklEQVR4nLWYyYtdRRTGf+++123HJJJ0kIgjGTQ0GNSWIA4g+h8oKAQEUSQo6MJFUDeKqOBecCNqgghmYfaC4s4okoDRoOBEsLPIZIhx6k6/+7moc7rqVdfrd9/ggeLeW7fq1KnvjFUtSYxBraQBCKjHYZhTZ8R5FUGoLkGolFr2vy78G5paIyDYNsEANgLbgVngX2AB+C0Z64K2kr7hFpTUtLWsIWmPpIOSTqmX/pL0haR9kjo2tsr4tK211GDdYYTzhd6Q1M0E60qqs74jkuZsznpJGwvC5t+rWlMVu1rfA54wtdXWn6vPHaUDnAd+BG60sReA48DHwGFgmV6TGUnFbXvuN2QWC2iVKEc5p6OS7svWGBpB9/LrgB+AKaIHNyFH00ORktYxBB8DDvVDslqDedsYLANPATPGuKlw2Ng2cVOVfXdMmDbwIXB38t3LoICg77YGdhB2+CwhlKRBeRLkQp0A5glgOMKBMp2n3vqCpEsNbG1cumzPR21dD09I6lFxmgHeBd4ENiS7+j9JwN7kfYWq7L0LvAY8CSwRjbmVTZ6kwG6fu4m2uWJGboOO3O3AMaJtTNLe+pE73lngZuAi0eNXEHRB9ifveQBeIjjMbcBJJl+5lPkp5sSrJJ0zg00Dsb+fkzRlYx+2vuUJOInz+FKF9FclKO4AtrA61nlZtQV4zvoOA58zKE01o8rWvAHYaShG31BMMw8YWqUUVVs7L2mrAurztvtSoTAqikcVtFTZGj1evEx/p3AUZ4FXbMfHgPeJDjYOedaaBx4nFiKkOt+mGDRLiNS20yVJ99gOr5V0NkF4XBRrSV8rqT3TivckIeX0805Hd4pQPgGcBv4gCQtjUBoPrzd+lau4bUK9s8ZiHhs/Az6yMc8QSv4u0bBrYpFRav3Mwde9guAwoU+9OXidpBMGeeos7jxLknbb+K0KTjMJ9Tq5wz1ocrW93vPQ8g8hJ35Kb8hxo30L+NbmvE5wGkfWx35PqJpzTfj3LkLGKpVuXi1dWOlR+YxwJDNcSbooabP9v0vRaZSMOS1pJuNZaj8niCnjcUbh/IKkVl7N1ISj5Dbrc8N129gDXA28Xdi9j5mz5zp7epux5y5gfQFBD/ifAJdcK2nB6oJsAn6iV8X+7NrkTYUF/HsROEV/uga4MpsvoqPdQTCRCqhLNwt/EiqKVEBn1DbhetNR3CAElLavIWC6GX+/DEwTSr3jJCm0yiZ5RP+Ocjz0cnyts4zP69dSrXj2mgYOAC8TQx65gCS7OsRqG/P/g2pEr8z7tbSc6xBM5kXCeXvVnU5+aPLJMwSod9LntDUGuRZ+JzjbQeAXYlXTmyQKIcCrm3uTwDmJus/J8/1LhTUbH9zdSPcCHxBtM78PdGcZ5iAvQkK4BThD9OA+MwZfedwv6ZsBqAy65pCCJhbt/flByA1CMEdyCngEeAi4Fdhs6C0QKvFZQ9jRzOOjx7iKUJDso2k1PmgHKl+ZbZS0wb7nJH1VQCtHdVnSqwmPid8P+sVjSfiOpKcVDj5/Z4ItSDog6c5hhWui4hLl6stL/psIt2HThFDyKyHW0Vit6WIjCFjkw9oX516ODX12mZSAKaVhJ70PHIn+AyfHLdi2OiXzAAAAAElFTkSuQmCC)}
.cell .moc{--m:url(data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAACgAAAAoCAYAAACM/rhtAAABS0lEQVR4nO2ZUUrEMBCGv9p9W0EWPIa+egw9hB5rT7HH8FXv4IuggvtmGx/aWcKSpJuZZulCPiiFdjLzk06mnbRxzhGhjd0oRBe62CQELoJV4t4DcAM4oCkUX3z/AK8hg9AMNuPAN+CukLBj3oF7L/aB1AzugZ4hN0rlo/jexwxSAq/Gw43nEojvqP9SgWcjNYMpOo5y5QQaFKmiFXi2GqkR6IAd8Etg1QUQm2vgkcySlSNQalYHvACfOYGAW+BjjHlybdU+4g3wzbDI+glbsdloAlkWyR95AoPv2ikWX2aqQCtVoJUq0EoVaKUKtFIFWqkCrVh6khV534OqPkYr8IvhgzV3TDY5AqWHaIEtuqap9a7NLtAP+KQYp6I27lYuusz03lFqA1N8R0tVSuCaia2xGRDf65hBSKAk/zPn3QL2Yx+46E30RfyG+Afvi0b+6ZIqdgAAAABJRU5ErkJggg==)}
.cell .lbl{position:absolute;left:6px;right:6px;bottom:10px;text-align:center;font-weight:600;font-size:clamp(10px,2.2vw,13px);letter-spacing:1px;text-transform:uppercase;white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
.cell .lbl.dim{color:var(--faint)}
.cell .badge{position:absolute;top:6px;right:6px;padding:1px 8px;border-radius:9px;font-size:10px;font-weight:600;letter-spacing:.5px;text-transform:uppercase;border:1px solid var(--faint);color:var(--muted);background:rgba(255,255,255,.04)}
.cell .badge:empty{display:none}
.cell .age{position:absolute;top:7px;left:9px;font-size:10px;font-weight:600;color:var(--muted)}
.strip{position:relative}
#ble{position:absolute;top:8px;right:12px}#ble .dot{width:10px;height:10px;border-radius:50%;background:var(--faint);display:inline-block}#ble .dot.on{background:var(--work)}
.cell.empty{border-color:var(--cdim)}.cell.empty .lbl{color:var(--faint)}
.cell.unknown,.cell.idle{border-color:var(--cline)}
.cell.working{border-color:var(--work);box-shadow:0 0 14px 1px rgba(163,230,53,.4)}
.cell.working .masc{background:var(--work);opacity:.8;animation:pulseo 1.2s ease-in-out infinite}
.cell.working .badge{border-color:var(--work);color:var(--work);background:rgba(163,230,53,.14)}
.cell.attention{border-color:var(--attn);box-shadow:0 0 14px 1px rgba(240,179,91,.5);animation:blink .5s steps(2,end) infinite}
.cell.attention .masc{background:var(--attn);opacity:.9}
.cell.attention .badge{border-color:var(--attn);color:var(--attn);background:rgba(240,179,91,.15)}
.cell.done{border-color:var(--ok);box-shadow:0 0 12px 1px rgba(120,140,93,.4)}
.cell.done .masc{background:var(--ok);opacity:.8}
.cell.done .badge{border-color:var(--ok);color:var(--ok);background:rgba(120,140,93,.15)}
.cell.error{border-color:var(--bad);box-shadow:0 0 12px 1px rgba(200,82,74,.45)}
.cell.error .masc{background:var(--bad)}
.cell.dead{opacity:.5}.cell.dead .lbl{text-decoration:line-through}
.cell.active{border-width:3px;border-color:var(--accent);box-shadow:0 0 16px 2px rgba(217,119,87,.45)}
.cell.active .masc{background:var(--accent);opacity:.95}
.cell.util .lbl{color:var(--accenthi)}
.cell.util .sub{position:absolute;top:8px;left:0;right:0;text-align:center;color:var(--muted)}
.strip{grid-column:1/-1;border-radius:16px;background:#10110F;border:2px solid #2A2B29;display:flex;align-items:center;justify-content:center;gap:16px}
.strip .clowmask{width:48px;height:38px;background:var(--accent);-webkit-mask:url(data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAADAAAAAmCAYAAACCjRgBAAAAtElEQVR4nO2Y2xKAIAhEs+n/f9mecxQEUUL3PDZ5WRYmIuWcL4bWC4lbKER1zm18ieVAgDfPwNoyZ6U1wRZfD0c4UEbWJHKC80iOcKA34lbOiGorvAMQ4E2tBrhcbuWktAZ69yFrIrwDEOANBHij6UZnfXFVbOHA7O7Sms99wzsAAd5AgDeUgHTZT980kPfY2oEQhBcwMplbNZ0mOdoB6b/zlJ5rCwe4XP7LbLT6PLwDEODNCxQSF1rNGV6oAAAAAElFTkSuQmCC) center/contain no-repeat;mask:url(data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAADAAAAAmCAYAAACCjRgBAAAAtElEQVR4nO2Y2xKAIAhEs+n/f9mecxQEUUL3PDZ5WRYmIuWcL4bWC4lbKER1zm18ieVAgDfPwNoyZ6U1wRZfD0c4UEbWJHKC80iOcKA34lbOiGorvAMQ4E2tBrhcbuWktAZ69yFrIrwDEOANBHij6UZnfXFVbOHA7O7Sms99wzsAAd5AgDeUgHTZT980kPfY2oEQhBcwMplbNZ0mOdoB6b/zlJ5rCwe4XP7LbLT6PLwDEODNCxQSF1rNGV6oAAAAAElFTkSuQmCC) center/contain no-repeat}
.strip b{font-size:18px;font-weight:600;letter-spacing:3px}
@keyframes pulseo{0%,100%{opacity:.55}50%{opacity:.95}}
@keyframes blink{0%{background:rgba(240,179,91,.2)}50%{background:var(--cell)}}
#pop{position:fixed;inset:0;background:rgba(0,0,0,.6);display:none;align-items:center;justify-content:center}
#pop.on{display:flex}#pop .box{background:#191A18;border:2px solid #3A3B39;border-radius:16px;padding:16px;min-width:280px;max-width:90vw}
#pop h3{margin:0 0 10px;font-size:15px}#pop h3 span{color:var(--muted);font-weight:400}
#pop button{display:block;width:100%;margin:6px 0;padding:10px;border:2px solid #2A2B29;border-radius:12px;background:var(--cell);color:var(--text);font-size:14px;cursor:pointer;text-align:left}
#pop button.danger{color:var(--bad);border-color:rgba(200,82,74,.5)}#pop button:hover{background:var(--press)}#pop button.hold{border-color:rgba(217,119,87,.6)}#pop button.hold.rec{background:#3a1f1a;border-color:var(--bad)}
#log{width:min(92vw,440px);height:170px;margin:0 auto 24px;padding:26px 12px 10px;position:relative;background:#0B0C0A;border:2px solid #2A2B29;border-radius:12px;color:var(--muted);font:11px/1.6 ui-monospace,Menlo,monospace;white-space:pre-wrap;overflow-y:auto;overflow-x:hidden}#log::before{content:'>_ log do agente';position:sticky;top:-16px;display:block;margin:-16px -12px 8px;padding:4px 12px;background:#141513;border-bottom:1px solid #232422;color:var(--faint);font:600 10px Montserrat,sans-serif;letter-spacing:1.5px;text-transform:uppercase}
.cell.action{display:flex;align-items:center;justify-content:center;text-align:center}
.cell.action .albl{font-weight:600;font-size:clamp(11px,2.4vw,14px);letter-spacing:1px;text-transform:uppercase;padding:4px}
.cell.action small{display:block;color:var(--muted);font-size:9px;letter-spacing:.5px;margin-top:4px;text-transform:none}
.cell.back .albl{color:var(--muted)}
.cell.accent{border-color:var(--accent)}.cell.accent .albl{color:var(--accenthi)}
.cell.ok{border-color:var(--ok)}.cell.ok .albl{color:#A8BD8B}
.cell.danger{border-color:rgba(200,82,74,.55)}.cell.danger .albl{color:var(--bad)}
.cell.voice{border-color:var(--cline)}
.cell.rec{border-color:var(--bad);box-shadow:0 0 16px 2px rgba(200,82,74,.5);animation:blink .5s steps(2,end) infinite}
.strip.sess{justify-content:flex-start;gap:14px;padding:0 52px 0 16px}
.strip .sinfo{display:flex;flex-direction:column;gap:3px;min-width:0;flex:1}
.strip .sinfo b{font-size:15px;letter-spacing:1px;text-transform:uppercase;white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
.strip .sinfo .st{font-size:11px;font-weight:600;letter-spacing:1px;text-transform:uppercase;color:var(--muted)}
.strip .sinfo .st.working{color:var(--work)}.strip .sinfo .st.attention{color:var(--attn)}.strip .sinfo .st.done{color:var(--ok)}.strip .sinfo .st.error{color:var(--bad)}
.strip .sinfo span:last-child{font-size:10px;color:var(--faint);text-transform:none}
.strip .masc.big{width:44px;height:44px;background:var(--faint);opacity:.85}
.strip .masc.big.working{background:var(--work)}.strip .masc.big.attention{background:var(--attn)}.strip .masc.big.done{background:var(--ok)}
#scmd{border:2px solid var(--accent);border-radius:18px;background:var(--cell);color:var(--accenthi);font:600 12px Montserrat,sans-serif;letter-spacing:1px;padding:9px 16px;cursor:pointer}
</style></head><body>
<div id="deck"></div>
<div id="pop"><div class="box"><h3 id="pt"></h3><div id="pb"></div></div></div>
<pre id="log"></pre>
<script>
// Espelha o deck fisico: retrato 3x4 — linhas 0-1 = celulas de sessao (N = layout.session_cells),
// linha 2 = 3 celulas utilitarias LOCAIS do deck (idioma, brilho, status), linha 3 = faixa livre.
const S={};const deck=document.getElementById('deck');const log=document.getElementById('log');
let builtKey='';
function el(t,c,h){const e=document.createElement(t);if(c)e.className=c;if(h!=null)e.innerHTML=h;return e}
function fmtAge(s){return s<60?s+'s':s<3600?Math.floor(s/60)+'m':Math.floor(s/3600)+'h'}
function say(m){const t=new Date().toLocaleTimeString();log.textContent=(t+'  '+m+'\n'+log.textContent).split('\n').slice(0,60).join('\n')}
async function send(kind,cell,arg){const r=await fetch('/event',{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify({kind,cell,arg})});const j=await r.json();say((j.ok?'ok  ':'ERR ')+(j.result||j.error));refresh();return j}
function pop(title,sub,items){const p=document.getElementById('pop');document.getElementById('pt').innerHTML=title+' <span>'+(sub||'')+'</span>';const b=document.getElementById('pb');b.innerHTML='';
 for(const it of items){const bt=el('button',(it.danger?'danger':'')+(it.hold?' hold':''),it.label);
  if(it.hold){bt.onpointerdown=e=>{e.preventDefault();bt.classList.add('rec');bt.textContent='gravando… solte para transcrever';it.down()};
   const up=()=>{if(!bt.classList.contains('rec'))return;bt.classList.remove('rec');bt.textContent=it.label;it.up()};bt.onpointerup=up;bt.onpointerleave=up;bt.onpointercancel=up}
  else bt.onclick=async()=>{if(it.confirm&&!confirm('Confirmar '+it.label+'?'))return;if(!it.keep)p.classList.remove('on');await it.run()};
  b.appendChild(bt)}
 const c=el('button','', 'fechar');c.onclick=()=>p.classList.remove('on');b.appendChild(c);p.classList.add('on')}
document.getElementById('pop').onclick=e=>{if(e.target.id==='pop')e.target.classList.remove('on')};
function infoMenu(){const b=S.ble||{};pop('Agente',S.version,[{label:'BLE: '+(b.enabled?(b.connected?'conectado a '+b.device:'procurando deck… '+(b.last_error||'')):'desligado (--no-ble)'),run:()=>{}},
 {label:'hooks recebidos: '+S.hooks_received+' · uptime '+fmtAge(S.uptime_s)+(S.dry_run?' · DRY-RUN':''),run:()=>{}},{label:'config: '+S.config_path,run:()=>{}},{label:'reenviar estado (HELLO)',run:()=>send('deck',1,0)}])}
let page='grid',pageCell=0,lastHint='';
function goto2(p,cell){page=p;if(cell!=null)pageCell=cell;builtKey='';render()}
function mkcell(lbl,cls,fn){const d=el('div','cell action '+(cls||''));d.innerHTML='<div class="albl">'+lbl+'</div>';if(fn)d.onclick=fn;deck.appendChild(d);return d}
function stripEl(html){const st=el('div','strip'+(html.indexOf('sinfo')>=0?' sess':''));st.id='strip';st.innerHTML=html;deck.appendChild(st);return st}
function gridShape(){const L=S.layout||{cols:3,rows:4};deck.innerHTML='';deck.style.gridTemplateColumns='repeat('+(L.cols||3)+',1fr)';deck.style.gridTemplateRows='repeat('+(L.rows||4)+',1fr)'}
function buildGrid(){const L=S.layout||{session_cells:6};const n=Math.min(L.session_cells||6,8);gridShape();
 for(let i=0;i<n;i++){const d=el('div','cell empty');d.dataset.cell=i;let timer=null,held=false;
  d.onpointerdown=()=>{held=false;timer=setTimeout(()=>{held=true;const c=S.cells?S.cells[i]:null;if(c&&c.sid)goto2('session',i)},500)};
  d.onpointerup=()=>{clearTimeout(timer);if(!held){const c=S.cells?S.cells[i]:null;if(c&&c.sid)send('cell_tap',i,0)}};d.onpointerleave=()=>clearTimeout(timer);d.oncontextmenu=e=>{e.preventDefault();const c=S.cells?S.cells[i]:null;if(c&&c.sid)goto2('session',i)};
  deck.appendChild(d)}
 const u=[['PT / EN','u-lang'],['brilho','u-bri'],['status','u-st']];
 u.forEach(([t,id])=>{const d=el('div','cell util');d.innerHTML='<div class="lbl">'+t+'</div><div class="sub"><span class="chip" id="'+id+'"></span></div>';if(id==='u-st')d.onclick=infoMenu;else d.title='local ao deck (config.toml [deck])';deck.appendChild(d)});
 stripEl('<i class="clowmask"></i><b>CLOW DECK</b><span id="ble"><i class="dot"></i></span>')}
function buildSession(){const c=(S.cells||[])[pageCell]||{};const t=pageCell;const cdx=!!(c.codex||c.opencode);gridShape();
 mkcell('&lt; voltar','back',()=>goto2('grid'));
 mkcell('focar','accent',()=>send('action',t,'focus'));
 if(cdx)mkcell('aprovar','ok',()=>send('action',t,'approve'));
 else{const v=mkcell('voz<br><small>segure p/ falar</small>','voice');
  v.onpointerdown=e=>{e.preventDefault();v.classList.add('rec');send('action',t,'voice_start')};
  const up=()=>{if(!v.classList.contains('rec'))return;v.classList.remove('rec');send('action',t,'voice_stop')};
  v.onpointerup=up;v.onpointerleave=up;v.onpointercancel=up}
 if(cdx)mkcell('negar','danger',()=>send('action',t,'esc'));else mkcell('modo',null,()=>send('action',t,'mode_cycle'));
 mkcell('esc',null,()=>send('action',t,'esc'));
 mkcell('enter',null,()=>send('action',t,'enter'));
 mkcell('tab','ok',()=>send('action',t,'tab'));
 mkcell('/compact',null,()=>send('action',t,'compact'));
 mkcell('&gt; mais','back',()=>goto2('cmd'));
 stripEl('<div class="sinfo"><b>'+(c.label||'')+'</b><span id="sst" class="st"></span><span id="sinfo2"></span></div><i id="smasc" class="masc big"></i><span id="ble"><i class="dot"></i></span>');
}
let cmdPage=0;
function buildCmd(){gridShape();const t=pageCell;const c=(S.cells||[])[t]||{};const cdx=!!(c.codex||c.opencode);
 mkcell('&lt; voltar','back',()=>{cmdPage=0;goto2('session')});
 const items=[
  {l:cdx?'/new':'/clear',cls:'danger',fn:()=>{if(confirm('Confirmar '+(cdx?'/new':'/clear')+'?'))send('action',t,'clear')}},
  {l:'/init',cls:'accent',fn:()=>send('action',t,'init')}];
 (S.commands||[]).forEach((cm,i)=>items.push({l:(cm.confirm?'! ':'')+cm.label,cls:null,fn:()=>{if(cm.confirm&&!confirm('Confirmar '+cm.label+'?'))return;send('action',t,'custom_'+i)}}));
 const per=7;const start=cmdPage*per;const more=(start+per)<items.length;
 items.slice(start,start+per).forEach(it=>mkcell(it.l,it.cls,it.fn));
 for(let k=items.slice(start,start+per).length;k<per;k++)deck.appendChild(el('div','cell empty','<div class="lbl dim">---</div>'));
 if(more)mkcell('&gt; mais','back',()=>{cmdPage++;builtKey='';render()});else deck.appendChild(el('div','cell empty','<div class="lbl dim">---</div>'));
 stripEl('<i class="clowmask"></i><b>COMANDOS</b><span id="ble"><i class="dot"></i></span>')}
function render(){const key=JSON.stringify([S.layout||{},page,pageCell]);
 if(key!==builtKey){if(page==='session')buildSession();else if(page==='cmd')buildCmd();else buildGrid();builtKey=key}
 if(page==='grid'){const cells=deck.querySelectorAll('.cell:not(.util)');const n=cells.length;
  for(let i=0;i<n;i++){const c=S.cells[i],d=cells[i];const st=c.sid?c.state_name:'empty';d.className='cell '+st+(c.active?' active':'')+(st==='done'&&c.age_s<60?' fresh':'');
   if(!c.sid){d.innerHTML='<div class="lbl dim">---</div>';continue}
   d.innerHTML='<span class="age">'+((c.state_name==='done'||c.state_name==='attention')?fmtAge(c.age_s):'')+'</span><span class="badge">'+(c.opencode?'OC':c.codex?'CDX':(c.no_hooks?'sem hooks':(c.mode_name!=='--'?c.mode_name:'')))+'</span><i class="masc '+(c.opencode?'moc':c.codex?'mcodex':'mclaude')+'"></i><div class="lbl" title="'+(c.cwd||'')+' · '+c.state_name+' · '+fmtAge(c.age_s)+'">'+c.label+'</div>'}
  const dk=S.deck||{};document.getElementById('u-lang').textContent=dk.lang===1?'EN':'PT';document.getElementById('u-bri').textContent=Math.round((dk.brightness||0)/255*100)+'%';
  document.getElementById('u-st').textContent=S.ble.enabled?(S.ble.connected?'deck ok':'sem deck'):'web'}
 else if(page==='session'){const c=(S.cells||[])[pageCell];if(!c||!c.sid){goto2('grid');return}
  const st=document.getElementById('sst');if(st){st.textContent=c.state_name;st.className='st '+c.state_name}
  const si=document.getElementById('sinfo2');if(si)si.textContent=(c.mode_name!=='--'?c.mode_name+' · ':'')+fmtAge(c.age_s)+' · célula '+(pageCell+1)+(c.active?' · ativa':'');
  const m=document.getElementById('smasc');if(m)m.className='masc big '+(c.opencode?'moc':c.codex?'mcodex':'mclaude')+' '+c.state_name}
 const bl=document.getElementById('ble');if(bl){bl.innerHTML='<i class="dot '+(S.ble.connected?'on':'')+'"></i>';bl.title=S.ble.enabled?(S.ble.connected?'deck: '+S.ble.device:'procurando deck'):'BLE off'}
 if(S.ble.hint&&S.ble.hint!==lastHint){say('aviso: '+S.ble.hint);lastHint=S.ble.hint}}
async function refresh(){try{Object.assign(S,await (await fetch('/state')).json());render()}catch(e){say('agente fora do ar')}}
refresh();setInterval(refresh,500);
</script></body></html>"##;
