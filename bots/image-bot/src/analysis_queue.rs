use tokio::sync::mpsc;

use crate::api_client::AnalyzeImageRequest;

/// Image en attente d'analyse dans la queue.
#[allow(dead_code)]
pub struct QueuedImage {
    pub request: AnalyzeImageRequest,
    pub channel_id: u64,
    pub message_id: u64,
    pub guild_id: String,
    pub author_name: String,
    pub author_face: String,
}

/// File d'attente pour l'analyse d'images (alternative a la suppression preventive).
pub struct AnalysisQueue {
    tx: mpsc::Sender<QueuedImage>,
}

impl AnalysisQueue {
    /// Cree une nouvelle queue avec la capacite specifiee.
    /// Retourne le sender (pour le bot) et le receiver (pour le consumer).
    pub fn new(capacity: usize) -> (Self, mpsc::Receiver<QueuedImage>) {
        let (tx, rx) = mpsc::channel(capacity);
        (Self { tx }, rx)
    }

    /// Ajoute une image a la queue. Retourne false si la queue est pleine.
    pub async fn enqueue(&self, image: QueuedImage) -> bool {
        self.tx.send(image).await.is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_request() -> AnalyzeImageRequest {
        AnalyzeImageRequest {
            guild_id: "1".to_string(),
            channel_id: "2".to_string(),
            user_id: "3".to_string(),
            username: "test".to_string(),
            message_id: "4".to_string(),
            image_data: b"base64data".to_vec(),
            content_type: "image/png".to_string(),
            filename: "test.png".to_string(),
            confidence_override: None,
            is_screenshot: false,
            is_animated: false,
        }
    }

    #[tokio::test]
    async fn enqueue_succeeds() {
        let (queue, _rx) = AnalysisQueue::new(10);
        let image = QueuedImage {
            request: make_request(),
            channel_id: 1,
            message_id: 2,
            guild_id: "1".to_string(),
            author_name: "test".to_string(),
            author_face: "url".to_string(),
        };
        assert!(queue.enqueue(image).await);
    }

    #[tokio::test]
    async fn enqueue_full_queue_fails() {
        let (queue, _rx) = AnalysisQueue::new(1);
        let img1 = QueuedImage {
            request: make_request(),
            channel_id: 1,
            message_id: 1,
            guild_id: "1".to_string(),
            author_name: "a".to_string(),
            author_face: "u".to_string(),
        };
        let img2 = QueuedImage {
            request: make_request(),
            channel_id: 1,
            message_id: 2,
            guild_id: "1".to_string(),
            author_name: "b".to_string(),
            author_face: "u".to_string(),
        };
        assert!(queue.enqueue(img1).await);
        // Queue de capacite 1, deja pleine → le second devrait echouer apres drop du rx
        drop(_rx);
        assert!(!queue.enqueue(img2).await);
    }
}
