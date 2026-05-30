use std::fmt;

use crate::{
    Context, api,
    message::{ApiError, FromService, Message},
};

#[derive(Debug)]
pub enum PathTreeError {
    IoError(String),
}

impl fmt::Display for PathTreeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PathTreeError::IoError(msg) => write!(f, "IoError: {msg}"),
        }
    }
}

/// Browse sub-directories of a given path
pub async fn get_sub_path(
    ctx: &mut Context<'_>,
    path: String,
    show_hide: bool,
) -> Result<Message<Vec<String>>, ApiError<PathTreeError>> {
    ctx.access_ctx.endpoint = api::Endpoint::GetSubPath;
    Message::from_service(ctx, async move {
        match std::fs::read_dir(&path) {
            Ok(dir) => {
                let result: Vec<_> = dir
                    .filter_map(|x| {
                        if let Ok(entry) = x
                            && let Ok(metadata) = entry.metadata()
                            && metadata.is_dir()
                            && let Some(dir_name) = entry.file_name().to_str()
                        {
                            if !show_hide
                                && let Ok(is_hide) = common::fs::is_hide(entry.path())
                                && is_hide
                            {
                                None
                            } else {
                                Some(dir_name.to_string())
                            }
                        } else {
                            None
                        }
                    })
                    .collect();
                Ok(result)
            }
            Err(err) => Err(PathTreeError::IoError(err.to_string())),
        }
    })
    .await
}
