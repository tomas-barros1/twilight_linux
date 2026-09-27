//! Detecção de serviços conflitantes de filtro de tela.

use std::process::Command;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConflictReport {
    pub name: String,
    pub detail: String,
}

/// Processos conhecidos que também aplicam temperatura/gama e podem brigar com sunsetr.
const CONFLICT_BINS: &[(&str, &str)] = &[
    ("hyprsunset", "hyprsunset (IPC Hyprland) — pode sobrescrever o CTM do sunsetr"),
    ("gammastep", "gammastep — filtro Wayland/X11 independente"),
    ("wlsunset", "wlsunset — filtro Wayland independente"),
    ("redshift", "redshift — filtro clássico (X11/Wayland)"),
    ("nightlight", "nightlight / outro filtro de noite"),
];

pub fn detect_conflicts(sunsetr_running: bool) -> Vec<ConflictReport> {
    let mut out = Vec::new();

    for &(bin, detail) in CONFLICT_BINS {
        if process_running(bin) {
            // hyprsunset só é conflito se estiver rodando em paralelo ao sunsetr
            // (sunsetr pode usá-lo como backend interno, mas tipicamente não deixa
            // o processo hyprsunset autônomo ativo ao mesmo tempo).
            if bin == "hyprsunset" && !sunsetr_running {
                continue;
            }
            out.push(ConflictReport {
                name: bin.to_string(),
                detail: detail.to_string(),
            });
        }
    }

    out
}

fn process_running(name: &str) -> bool {
    Command::new("pgrep")
        .args(["-x", name])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_when_nothing_running() {
        // Não afirma ausência global; só garante que a API não pânico.
        let _ = detect_conflicts(false);
    }
}
