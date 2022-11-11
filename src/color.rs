/// Color represented in HSL
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Default)]
pub struct Hsl {
    /// Hue in 0-360 degree
    pub h: f64,
    /// Saturation in 0...1 (percent)
    pub s: f64,
    /// Luminosity in 0...1 (percent)
    pub l: f64,
}

impl Hsl {
    pub fn new(h: f32, s: f32, l: f32) -> Hsl {
        Hsl {
            h: h as _,
            s: s as _,
            l: l as _,
        }
    }
    pub fn hsl_to_rgb(&self) -> (u8, u8, u8) {
        if self.s == 0.0 {
            // Achromatic, i.e., grey.
            let l = percent_to_byte(self.l);
            return (l, l, l);
        }
    
        let h = self.h / 360.0; // treat this as 0..1 instead of degrees
        let s = self.s;
        let l = self.l;
    
        let q = if l < 0.5 {
            l * (1.0 + s)
        } else {
            l + s - (l * s)
        };
        let p = 2.0 * l - q;
    
        (percent_to_byte(hue_to_rgb(p, q, h + 1.0 / 3.0)),
         percent_to_byte(hue_to_rgb(p, q, h)),
         percent_to_byte(hue_to_rgb(p, q, h - 1.0 / 3.0)))
    }
}


fn percent_to_byte(percent: f64) -> u8 {
    (percent * 255.0).round() as u8
}

/// Convert Hue to RGB Ratio
///
/// From <https://github.com/jariz/vibrant.js/> by Jari Zwarts
fn hue_to_rgb(p: f64, q: f64, t: f64) -> f64 {
    // Normalize
    let t = if t < 0.0 {
        t + 1.0
    } else if t > 1.0 {
        t - 1.0
    } else {
        t
    };

    if t < 1.0 / 6.0 {
        p + (q - p) * 6.0 * t
    } else if t < 1.0 / 2.0 {
        q
    } else if t < 2.0 / 3.0 {
        p + (q - p) * (2.0 / 3.0 - t) * 6.0
    } else {
        p
    }
}