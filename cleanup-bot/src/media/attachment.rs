use serenity::all::Attachment;

/// Information about a media attachment that needs to be backed up.
#[derive(Debug, Clone)]
pub struct MediaAttachment {
    pub url: String,
    pub filename: String,
}

pub trait AttachmentsExt {
    fn extract_media(&self) -> Vec<MediaAttachment>;
}

/// Check if an attachment is a media file (image or video).
pub fn is_media(attachment: &Attachment) -> bool {
    attachment
        .content_type
        .as_ref()
        .map(|content_type| content_type.starts_with("image") || content_type.starts_with("video"))
        .unwrap_or(false)
}

impl AttachmentsExt for Vec<Attachment> {
    /// Extract media attachments from a list of attachments.
    fn extract_media(&self) -> Vec<MediaAttachment> {
        self.iter()
            .filter_map(|a| {
                if is_media(a) {
                    Some(MediaAttachment {
                        url: a.url.clone(),
                        filename: a.filename.clone(),
                    })
                } else {
                    None
                }
            })
            .collect()
    }
}
