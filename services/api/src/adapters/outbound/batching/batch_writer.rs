//! BatchWriter<T> — buffer in-memory + flush periodique pour inserts batch.
//!
//! # Architecture
//!
//! - `mpsc::channel` bornee (drop si plein → pas de blocage du request path)
//! - Flusher task spawn une fois au demarrage, consume la Receiver
//! - Deux triggers de flush : taille batch atteinte OR interval tick
//! - Sur drop du dernier Sender (shutdown API), le flusher draine le reste et exit
//!
//! # Configuration typique
//!
//! ```ignore
//! BatchWriterConfig {
//!     flush_interval: Duration::from_millis(500),
//!     max_batch_size: 100,
//!     channel_capacity: 10_000,
//! }
//! ```

use std::future::Future;
use std::time::Duration;

use tokio::sync::mpsc;
use tokio::time::MissedTickBehavior;
use tracing::{debug, warn};

#[derive(Debug, Clone, Copy)]
pub struct BatchWriterConfig {
    /// Intervalle max entre deux flushs (meme si le batch n'est pas plein).
    pub flush_interval: Duration,
    /// Taille max d'un batch avant flush immediat.
    pub max_batch_size: usize,
    /// Capacite du channel mpsc. Si plein, les envois sont drop.
    pub channel_capacity: usize,
}

impl Default for BatchWriterConfig {
    fn default() -> Self {
        Self {
            flush_interval: Duration::from_millis(500),
            max_batch_size: 100,
            channel_capacity: 10_000,
        }
    }
}

/// Handle cote producteur : wrap un `mpsc::Sender` avec la politique "drop si plein".
#[derive(Clone)]
pub struct BatchWriter<T: Send + 'static> {
    tx: mpsc::Sender<T>,
    label: &'static str,
}

impl<T: Send + 'static> BatchWriter<T> {
    /// Cree un BatchWriter + spawn la flusher task en background.
    ///
    /// `label` est utilise dans les logs (ex: "logs", "audit_logs").
    /// `flush_fn` recoit un `Vec<T>` non-vide a chaque flush, doit retourner
    /// `Ok(())` ou une erreur (qui sera logguee, sans retry — les entries sont perdues).
    pub fn spawn<F, Fut>(
        label: &'static str,
        config: BatchWriterConfig,
        flush_fn: F,
    ) -> Self
    where
        F: Fn(Vec<T>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), String>> + Send + 'static,
    {
        let (tx, rx) = mpsc::channel::<T>(config.channel_capacity);

        tokio::spawn(run_flusher(label, rx, config, flush_fn));

        Self { tx, label }
    }

    /// Enqueue un item non-bloquant. Retourne `true` si ajoute, `false` si drop (queue pleine).
    pub fn try_send(&self, item: T) -> bool {
        match self.tx.try_send(item) {
            Ok(()) => true,
            Err(mpsc::error::TrySendError::Full(_)) => {
                warn!(label = %self.label, "BatchWriter queue pleine — entry dropped");
                false
            }
            Err(mpsc::error::TrySendError::Closed(_)) => {
                warn!(label = %self.label, "BatchWriter channel ferme — entry dropped");
                false
            }
        }
    }
}

async fn run_flusher<T, F, Fut>(
    label: &'static str,
    mut rx: mpsc::Receiver<T>,
    config: BatchWriterConfig,
    flush_fn: F,
) where
    T: Send + 'static,
    F: Fn(Vec<T>) -> Fut + Send + Sync,
    Fut: Future<Output = Result<(), String>> + Send,
{
    let mut buffer: Vec<T> = Vec::with_capacity(config.max_batch_size);
    let mut interval = tokio::time::interval(config.flush_interval);
    interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
    // Premier tick immediat — on le consomme pour ne pas flush un batch vide au start.
    interval.tick().await;

    debug!(label = %label, "BatchWriter flusher demarre");

    loop {
        tokio::select! {
            _ = interval.tick() => {
                if !buffer.is_empty() {
                    let batch = std::mem::replace(&mut buffer, Vec::with_capacity(config.max_batch_size));
                    do_flush(label, &flush_fn, batch).await;
                }
            }
            maybe_item = rx.recv() => {
                match maybe_item {
                    Some(item) => {
                        buffer.push(item);
                        // Drainer sans attendre pour remplir le batch d'un coup
                        while buffer.len() < config.max_batch_size {
                            match rx.try_recv() {
                                Ok(next) => buffer.push(next),
                                Err(_) => break,
                            }
                        }
                        if buffer.len() >= config.max_batch_size {
                            let batch = std::mem::replace(&mut buffer, Vec::with_capacity(config.max_batch_size));
                            do_flush(label, &flush_fn, batch).await;
                        }
                    }
                    None => {
                        // Channel ferme → flush final et exit
                        if !buffer.is_empty() {
                            let batch = std::mem::take(&mut buffer);
                            do_flush(label, &flush_fn, batch).await;
                        }
                        debug!(label = %label, "BatchWriter flusher arrete (channel closed)");
                        return;
                    }
                }
            }
        }
    }
}

async fn do_flush<T, F, Fut>(label: &'static str, flush_fn: &F, batch: Vec<T>)
where
    T: Send + 'static,
    F: Fn(Vec<T>) -> Fut,
    Fut: Future<Output = Result<(), String>> + Send,
{
    let count = batch.len();
    let start = std::time::Instant::now();
    match flush_fn(batch).await {
        Ok(()) => {
            debug!(
                label = %label,
                count,
                elapsed_ms = start.elapsed().as_millis() as u64,
                "Batch flushed"
            );
        }
        Err(e) => {
            warn!(label = %label, count, error = %e, "Batch flush failed (entries perdues)");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tokio::sync::Mutex;

    #[tokio::test]
    async fn flushes_when_batch_full() {
        let flushed = Arc::new(Mutex::new(Vec::<Vec<u32>>::new()));
        let flushed_clone = flushed.clone();

        let writer = BatchWriter::spawn(
            "test",
            BatchWriterConfig {
                flush_interval: Duration::from_secs(60), // tres long pour ne pas trigger
                max_batch_size: 3,
                channel_capacity: 100,
            },
            move |batch: Vec<u32>| {
                let store = flushed_clone.clone();
                async move {
                    store.lock().await.push(batch);
                    Ok(())
                }
            },
        );

        for i in 0..5u32 {
            assert!(writer.try_send(i));
        }

        // Laisser le flusher tourner un peu
        tokio::time::sleep(Duration::from_millis(50)).await;

        let guard = flushed.lock().await;
        assert_eq!(guard.len(), 1, "un batch de 3 doit avoir ete flush");
        assert_eq!(guard[0], vec![0, 1, 2]);
    }

    #[tokio::test]
    async fn flushes_on_interval() {
        let flushed = Arc::new(Mutex::new(Vec::<Vec<u32>>::new()));
        let flushed_clone = flushed.clone();

        let writer = BatchWriter::spawn(
            "test",
            BatchWriterConfig {
                flush_interval: Duration::from_millis(50),
                max_batch_size: 1000, // tres grand pour ne pas trigger par taille
                channel_capacity: 100,
            },
            move |batch: Vec<u32>| {
                let store = flushed_clone.clone();
                async move {
                    store.lock().await.push(batch);
                    Ok(())
                }
            },
        );

        writer.try_send(42);
        writer.try_send(43);

        tokio::time::sleep(Duration::from_millis(150)).await;

        let guard = flushed.lock().await;
        assert!(!guard.is_empty(), "le tick doit avoir flush le batch partiel");
        let all: Vec<u32> = guard.iter().flatten().copied().collect();
        assert_eq!(all, vec![42, 43]);
    }

    #[tokio::test]
    async fn drains_on_channel_close() {
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let writer = BatchWriter::spawn(
            "test",
            BatchWriterConfig {
                flush_interval: Duration::from_secs(60),
                max_batch_size: 1000,
                channel_capacity: 100,
            },
            move |batch: Vec<u32>| {
                let c = counter_clone.clone();
                async move {
                    c.fetch_add(batch.len(), Ordering::SeqCst);
                    Ok(())
                }
            },
        );

        writer.try_send(1);
        writer.try_send(2);
        writer.try_send(3);

        drop(writer);

        // Attendre que le flusher draine et exit
        tokio::time::sleep(Duration::from_millis(100)).await;

        assert_eq!(counter.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn flush_error_does_not_stop_loop() {
        // Verifie que si flush_fn retourne Err, le flusher continue a
        // accepter des nouveaux items et ne panique pas. Les entries du
        // batch qui a echoue sont perdues (at-most-once) mais les suivants
        // sont quand meme traites.
        let call_count = Arc::new(AtomicUsize::new(0));
        let success_count = Arc::new(AtomicUsize::new(0));
        let cc = call_count.clone();
        let sc = success_count.clone();

        let writer = BatchWriter::spawn(
            "test",
            BatchWriterConfig {
                flush_interval: Duration::from_millis(30),
                max_batch_size: 2,
                channel_capacity: 100,
            },
            move |batch: Vec<u32>| {
                let cc = cc.clone();
                let sc = sc.clone();
                async move {
                    let n = cc.fetch_add(1, Ordering::SeqCst);
                    // Premier batch : erreur simulée. Suivants : succès.
                    if n == 0 {
                        Err("simulated db error".to_string())
                    } else {
                        sc.fetch_add(batch.len(), Ordering::SeqCst);
                        Ok(())
                    }
                }
            },
        );

        // Premier batch (2 items) → va échouer
        writer.try_send(1);
        writer.try_send(2);
        tokio::time::sleep(Duration::from_millis(60)).await;

        // Second batch (2 items) → doit réussir malgré l'échec précédent
        writer.try_send(3);
        writer.try_send(4);
        tokio::time::sleep(Duration::from_millis(60)).await;

        // Au moins 2 appels à flush_fn, et le second a bien traité 2 items
        assert!(call_count.load(Ordering::SeqCst) >= 2, "flush_fn doit etre rappele apres un echec");
        assert_eq!(success_count.load(Ordering::SeqCst), 2, "second batch doit etre persiste");
    }

    #[tokio::test]
    async fn try_send_returns_false_when_channel_full() {
        // Config extrême : capacité 1, max_batch_size énorme, interval très long
        // → le canal va se remplir avant que le flusher puisse drainer.
        let writer = BatchWriter::spawn(
            "test",
            BatchWriterConfig {
                flush_interval: Duration::from_secs(60),
                max_batch_size: 1_000,
                channel_capacity: 1,
            },
            move |_batch: Vec<u32>| async move {
                // Flush extrêmement lent pour laisser le canal saturer
                tokio::time::sleep(Duration::from_millis(500)).await;
                Ok(())
            },
        );

        // Bourre le canal — au moins un send doit echouer
        let mut dropped = 0;
        for i in 0..20u32 {
            if !writer.try_send(i) {
                dropped += 1;
            }
        }
        assert!(dropped > 0, "au moins un try_send doit renvoyer false quand le canal est plein");
    }
}
