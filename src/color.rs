//! Conversão Kelvin → RGB para o preview de temperatura.

/// Algoritmo aproximado de Tanner Helland (temperatura de cor → sRGB 0–1).
pub fn kelvin_to_rgb(kelvin: u32) -> (f64, f64, f64) {
    let temp = (kelvin.clamp(1000, 40000) as f64) / 100.0;

    let r = if temp <= 66.0 {
        1.0
    } else {
        let r = 1.292936186_f64.mul_add((temp - 60.0).ln(), -0.095093858);
        r.clamp(0.0, 1.0)
    };

    let g = if temp <= 66.0 {
        let g = 0.390081578_f64.mul_add(temp.ln(), -0.631841443);
        g.clamp(0.0, 1.0)
    } else {
        let g = 1.129890860_f64.mul_add((temp - 60.0).ln(), -0.169679873);
        g.clamp(0.0, 1.0)
    };

    let b = if temp >= 66.0 {
        1.0
    } else if temp <= 19.0 {
        0.0
    } else {
        let b = 0.543206789_f64.mul_add((temp - 10.0).ln(), -1.196254089);
        b.clamp(0.0, 1.0)
    };

    (r, g, b)
}

#[allow(dead_code)]
pub fn kelvin_to_css(kelvin: u32) -> String {
    let (r, g, b) = kelvin_to_rgb(kelvin);
    format!(
        "rgb({},{},{})",
        (r * 255.0).round() as u8,
        (g * 255.0).round() as u8,
        (b * 255.0).round() as u8
    )
}

/// Intensidade 0–100 ↔ temperatura (Twilight-like).
/// 0% ≈ 6500 K (neutro), 100% ≈ 2700 K (quente).
pub fn intensity_to_kelvin(intensity: u32) -> u32 {
    let i = (intensity.min(100) as f64) / 100.0;
    (6500.0 - i * 3800.0).round() as u32
}

pub fn kelvin_to_intensity(kelvin: u32) -> u32 {
    let k = kelvin.clamp(2700, 6500) as f64;
    let i = ((6500.0 - k) / 3800.0 * 100.0).round();
    i.clamp(0.0, 100.0) as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn daylight_is_neutralish() {
        let (r, g, b) = kelvin_to_rgb(6500);
        assert!((r - 1.0).abs() < 0.05);
        assert!(b > 0.9);
        assert!(g > 0.9);
    }

    #[test]
    fn candle_is_warm() {
        let (r, _g, b) = kelvin_to_rgb(2700);
        assert!(r > 0.9);
        assert!(b < 0.6);
    }

    #[test]
    fn intensity_roundtrip() {
        for i in [0, 25, 50, 75, 100] {
            let k = intensity_to_kelvin(i);
            let back = kelvin_to_intensity(k);
            assert!((back as i32 - i as i32).abs() <= 1, "{i} -> {k} -> {back}");
        }
    }
}
