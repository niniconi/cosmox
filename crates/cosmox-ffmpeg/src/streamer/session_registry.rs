//! # Session Registry Module (Transcoding Status Monitoring & Dynamic Tuning)
//!
//! Tracks active transcoding sessions, their FPS, hardware resource usage, and
//! provides cleanup for stale sessions.

use std::{
    collections::HashMap,
    error::Error,
    path::PathBuf,
    sync::{Arc, LazyLock},
    time::{Duration, Instant},
};

use tokio::sync::{RwLock, oneshot};

pub struct LiveStreamPipeline {
    pub session_id: String,
    pub media_id: String,
    pub output_dir: PathBuf,
    pub last_heartbeat: Instant,
    /// Channel sender to signal the blocking ffmpeg loop to stop.
    pub stop_tx: Option<oneshot::Sender<()>>,
}

#[derive(Clone)]
pub struct StreamSessionRegistry {
    pub active_sessions: Arc<RwLock<HashMap<String, LiveStreamPipeline>>>,
}

static STREAM_SEESION_REGISTRY: LazyLock<StreamSessionRegistry> =
    LazyLock::new(|| StreamSessionRegistry {
        active_sessions: Arc::new(RwLock::new(HashMap::new())),
    });

impl StreamSessionRegistry {
    pub fn global() -> &'static Self {
        &STREAM_SEESION_REGISTRY
    }

    pub fn remote_task() -> Result<(), Box<dyn Error>> {
        Ok(())
    }

    /// Push a new live transcoding task into the registry.
    pub async fn push_task(
        &self,
        session_id: String,
        media_id: String,
        input_path: String,
        output_dir: PathBuf,
    ) -> Result<(), Box<dyn Error>> {
        let (stop_tx, stop_rx) = oneshot::channel::<()>();
        let pipeline = LiveStreamPipeline {
            session_id: session_id.clone(),
            media_id,
            output_dir: output_dir.clone(),
            last_heartbeat: Instant::now(),
            stop_tx: Some(stop_tx),
        };

        // Register the task in the global session map.
        {
            let mut lock = self.active_sessions.write().await;
            lock.insert(session_id.clone(), pipeline);
        }

        // Spawn the blocking ffmpeg transcode pipeline via spawn_blocking.
        tokio::task::spawn_blocking(move || {
            println!(
                "Task [{}] launched on Tokio blocking thread pool.",
                session_id
            );

            // Each session gets its own temporary output directory.
            if let Err(e) = std::fs::create_dir_all(&output_dir) {
                eprintln!("Failed to create session temp directory: {:?}", e);
                return;
            }

            // TODO: Call the actual ffmpeg transcode pipeline from transcode.rs.
            // The `stop_rx` receiver should be polled on each loop iteration
            // to check whether the session should be terminated.
            // if let Err(e) =
            //   crate::streamer::transcode::run_hls_pipeline(&input_path, &output_dir, stop_rx)
            // {
            //   eprintln!("Task [{}] ffmpeg pipeline exited with error: {:?}", session_id, e);
            // }

            println!(
                "Task [{}] blocking thread finished, C pointers released.",
                session_id
            );
        });

        Ok(())
    }

    /// Refresh heartbeat for a session (called when the client fetches segments).
    pub async fn inspect_task(&self, session_id: &str) -> Result<(), Box<dyn Error>> {
        let mut lock = self.active_sessions.write().await;
        if let Some(pipeline) = lock.get_mut(session_id) {
            pipeline.last_heartbeat = Instant::now();
            Ok(())
        } else {
            Err(format!(
                "Session {} does not exist or has been destroyed",
                session_id
            )
            .into())
        }
    }

    /// Explicitly remove and destroy a task (e.g. client stop request).
    pub async fn remove_task(&self, session_id: &str) -> Result<(), Box<dyn Error>> {
        let mut lock = self.active_sessions.write().await;
        if let Some(mut pipeline) = lock.remove(session_id) {
            // Signal the transcoding loop to stop via oneshot channel.
            if let Some(tx) = pipeline.stop_tx.take() {
                let _ = tx.send(());
            }
            // Remove the temporary output directory to free disk space.
            if pipeline.output_dir.exists() {
                tokio::fs::remove_dir_all(&pipeline.output_dir).await?;
            }
            println!(
                "Task [{}] has been safely destroyed and cleaned up.",
                session_id
            );
        }
        Ok(())
    }

    /// Background reaper coroutine that periodically cleans up stale sessions.
    async fn start_reaper_loop(active_sessions: Arc<RwLock<HashMap<String, LiveStreamPipeline>>>) {
        loop {
            tokio::time::sleep(Duration::from_secs(5)).await;

            let mut lock = active_sessions.write().await;
            let now = Instant::now();
            let mut dead_session_ids = vec![];

            // Find sessions with heartbeat older than 20 seconds (presumed dead).
            for (id, pipeline) in lock.iter() {
                if now.duration_since(pipeline.last_heartbeat).as_secs() > 20 {
                    dead_session_ids.push(id.clone());
                }
            }

            // Perform cleanup on dead sessions.
            for id in dead_session_ids {
                if let Some(mut dead_pipeline) = lock.remove(&id) {
                    // Stop the transcoding pipeline.
                    if let Some(tx) = dead_pipeline.stop_tx.take() {
                        let _ = tx.send(());
                    }
                    // Remove the temp directory.
                    if dead_pipeline.output_dir.exists() {
                        let _ = tokio::fs::remove_dir_all(&dead_pipeline.output_dir).await;
                    }
                    println!(
                        "[Reaper] Client offline timeout, cleaned up session: {}",
                        id
                    );
                }
            }
        }
    }
}
