//! Lavori pesanti di generazione su tutti i core (ADR 0056).
//!
//! Il `ComputeTaskPool` di Bevy divide il lavoro in pezzi indipendenti: i risultati tornano
//! nell'ordine dei pezzi, quindi l'esito è identico al calcolo in serie.

use bevy::tasks::{ComputeTaskPool, TaskPool};

/// Il pool di calcolo di Bevy (creato al volo nei test, dove l'app non c'è).
fn pool() -> &'static ComputeTaskPool {
    ComputeTaskPool::get_or_init(TaskPool::default)
}

/// `f(i)` per `i` in `0..n`, in parallelo, con i risultati nell'ordine di `i`.
pub fn map<T, F>(n: usize, f: F) -> Vec<T>
where
    T: Send + 'static,
    F: Fn(usize) -> T + Sync,
{
    if n <= 1 {
        return (0..n).map(f).collect();
    }
    let f = &f;
    pool().scope(|s| {
        for i in 0..n {
            s.spawn(async move { f(i) });
        }
    })
}

/// `(0..len).map(f).collect()` calcolato a blocchi su tutti i core, stesso ordine e stessi valori.
pub fn collect<T, F>(len: usize, f: F) -> Vec<T>
where
    T: Send + 'static,
    F: Fn(usize) -> T + Sync,
{
    let threads = pool().thread_num().max(1);
    // Pochi elementi: non conviene dividere.
    if len < 4096 || threads == 1 {
        return (0..len).map(f).collect();
    }
    let chunk = len.div_ceil(threads * 4);
    let parts = map(len.div_ceil(chunk), |c| {
        let start = c * chunk;
        (start..(start + chunk).min(len))
            .map(&f)
            .collect::<Vec<T>>()
    });
    let mut out = Vec::with_capacity(len);
    for p in parts {
        out.extend(p);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collect_matches_serial_order_and_values() {
        let f = |i: usize| (i as f32 * 0.37).sin() * i as f32;
        for len in [0, 1, 7, 4095, 4096, 100_003] {
            let serial: Vec<f32> = (0..len).map(f).collect();
            assert_eq!(collect(len, f), serial, "len {len}");
        }
    }

    #[test]
    fn map_keeps_order() {
        assert_eq!(
            map(10, |i| i * i),
            (0..10).map(|i| i * i).collect::<Vec<_>>()
        );
    }
}
