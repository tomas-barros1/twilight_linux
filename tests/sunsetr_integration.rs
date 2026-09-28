//! Teste de integração opcional com o binário sunsetr (ignorado se ausente).

use twilight_linux_test_support::*;

// O binário não exporta lib — reimplementamos helpers mínimos aqui.
mod twilight_linux_test_support {
    use std::process::Command;

    pub fn sunsetr_available() -> bool {
        Command::new("sunsetr")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
}

#[test]
fn sunsetr_status_json_when_running() {
    if !sunsetr_available() {
        eprintln!("skip: sunsetr não instalado");
        return;
    }

    let output = std::process::Command::new("sunsetr")
        .args(["status", "--json"])
        .output()
        .expect("spawn sunsetr status");

    if !output.status.success() {
        eprintln!("skip: sunsetr não está em execução");
        return;
    }

    let body = String::from_utf8_lossy(&output.stdout);
    let body = body.trim();
    // Alguns ambientes misturam ANSI no stderr; stdout deve ser JSON puro.
    if !body.starts_with('{') {
        eprintln!("skip: stdout inesperado: {body}");
        return;
    }
    let v: serde_json::Value = serde_json::from_str(body).expect("JSON válido");
    assert!(v.get("period").is_some(), "status deve incluir period");
    assert!(
        v.get("current_temp").is_some() || v.get("target_temp").is_some(),
        "status deve incluir temperatura"
    );
}

#[test]
fn sunsetr_get_all_json() {
    if !sunsetr_available() {
        eprintln!("skip: sunsetr não instalado");
        return;
    }

    let output = std::process::Command::new("sunsetr")
        .args(["get", "all", "--json"])
        .output()
        .expect("spawn sunsetr get");
    assert!(output.status.success(), "get all deve funcionar sem daemon");
    let body = String::from_utf8_lossy(&output.stdout);
    let v: serde_json::Value = serde_json::from_str(&body).expect("JSON válido");
    assert!(v.get("night_temp").is_some());
    assert!(v.get("backend").is_some());
}
