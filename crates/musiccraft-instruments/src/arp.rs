//! The arpeggiator's key order, shared by the app (which arpeggiates when it
//! builds a sequence) and the plugins (which arpeggiate in real time).
//! Allocation-free.

/// Most keys one arpeggio cycles through: every MIDI key once, plus the way
/// back down in up-down mode.
pub const MAX_ORDER: usize = 256;

/// A stable pseudo-random number for a step, so random order repeats.
#[must_use]
pub fn hash(step: u64) -> u64 {
    let mut x = step.wrapping_add(0x9E37_79B9_7F4A_7C15);
    x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^ (x >> 31)
}

/// Writes the keys to cycle through, in play order, for the held `keys`:
/// sorted, spread over `range` octaves, reversed for mode 2 (down) and
/// folded back for mode 3 (up-down). Returns how many it wrote.
pub fn order(keys: &[u8], mode: u8, range: u8, out: &mut [u8; MAX_ORDER]) -> usize {
    // Which keys sound, as a set over the MIDI range.
    let mut on = [false; 128];
    for key in keys {
        for octave in 0..range.max(1) {
            if let Some(slot) = key
                .checked_add(12 * octave)
                .and_then(|k| on.get_mut(usize::from(k)))
            {
                *slot = true;
            }
        }
    }
    let mut len = 0;
    for (key, _) in on.iter().enumerate().filter(|(_, on)| **on) {
        if let Some(slot) = out.get_mut(len) {
            *slot = key as u8;
            len += 1;
        }
    }
    match mode {
        2 => {
            if let Some(spread) = out.get_mut(..len) {
                spread.reverse();
            }
        }
        3 if len > 2 => {
            for i in (1..len - 1).rev() {
                let key = out.get(i).copied().unwrap_or(0);
                if let Some(slot) = out.get_mut(len + (len - 2 - i)) {
                    *slot = key;
                }
            }
            len += len - 2;
        }
        _ => {}
    }
    len.min(MAX_ORDER)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keys(held: &[u8], mode: u8, range: u8) -> Vec<u8> {
        let mut out = [0; MAX_ORDER];
        let n = order(held, mode, range, &mut out);
        out[..n].to_vec()
    }

    #[test]
    fn modes_and_octaves() {
        assert_eq!(keys(&[64, 60, 67, 60], 1, 1), [60, 64, 67]);
        assert_eq!(keys(&[60, 64, 67], 2, 1), [67, 64, 60]);
        assert_eq!(keys(&[60, 64, 67], 3, 1), [60, 64, 67, 64]);
        assert_eq!(keys(&[60, 64], 3, 1), [60, 64]);
        assert_eq!(keys(&[60, 64, 67], 1, 2), [60, 64, 67, 72, 76, 79]);
        assert_eq!(keys(&[120], 1, 3), [120], "nothing past key 127");
        // Every key over four octaves still fits.
        let all: Vec<u8> = (0..128).collect();
        assert_eq!(keys(&all, 3, 4).len(), 128 + 126);
    }
}
