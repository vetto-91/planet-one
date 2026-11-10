//! Ora locale della macchina (salvataggi, debug e log), senza dipendenze pesanti.
//!
//! Su Linux e macOS si usa `localtime_r` della libreria C; altrove si ripiega sull'ora UTC.

/// Data e ora: anno, mese, giorno, ore, minuti, secondi.
pub type Stamp = (i32, u32, u32, u32, u32, u32);

/// Data civile UTC da secondi dal 1970 (algoritmo di Howard Hinnant).
pub fn civil_from_secs(secs: i64) -> Stamp {
    let (days, rem) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (
        y as i32,
        m as u32,
        d as u32,
        (rem / 3600) as u32,
        (rem % 3600 / 60) as u32,
        (rem % 60) as u32,
    )
}

/// Ora locale (o UTC se il sistema non la dà).
pub fn now() -> Stamp {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs()) as i64;
    #[cfg(unix)]
    {
        // SAFETY: `localtime_r` scrive solo nella struttura passata e non tiene puntatori.
        unsafe {
            let t = secs as libc::time_t;
            let mut tm: libc::tm = std::mem::zeroed();
            if !libc::localtime_r(&t, &mut tm).is_null() {
                return (
                    tm.tm_year + 1900,
                    (tm.tm_mon + 1) as u32,
                    tm.tm_mday as u32,
                    tm.tm_hour as u32,
                    tm.tm_min as u32,
                    tm.tm_sec as u32,
                );
            }
        }
    }
    civil_from_secs(secs)
}

/// «2026-11-09 21:42» in ora locale.
pub fn minute_text() -> String {
    let (y, m, d, h, mi, _) = now();
    format!("{y:04}-{m:02}-{d:02} {h:02}:{mi:02}")
}

/// Orologio per i log di tracing.
pub struct LogClock;

impl bevy::log::tracing_subscriber::fmt::time::FormatTime for LogClock {
    fn format_time(
        &self,
        w: &mut bevy::log::tracing_subscriber::fmt::format::Writer<'_>,
    ) -> std::fmt::Result {
        let (y, m, d, h, mi, s) = now();
        write!(w, "{y:04}-{m:02}-{d:02} {h:02}:{mi:02}:{s:02}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn civil_date_matches_known_instants() {
        assert_eq!(civil_from_secs(0), (1970, 1, 1, 0, 0, 0));
        assert_eq!(civil_from_secs(1_794_249_000), (2026, 11, 9, 18, 30, 0));
        assert_eq!(civil_from_secs(951_782_400 + 3_661), (2000, 2, 29, 1, 1, 1));
    }

    #[test]
    fn local_time_is_close_to_utc_clock() {
        let (y, ..) = now();
        assert!(y >= 2026);
        assert_eq!(minute_text().len(), 16);
    }
}
