use std::slice;

#[derive(Debug)]
pub struct VideoInfo {
    pub url: String,
    pub thumbnail: Option<String>,
    pub title: String,
}

#[derive(Debug)]
pub struct PlaylistInfo {
    pub url: String,
    pub thumbnail: Option<String>,
    pub title: String,
    pub videos: Vec<VideoInfo>,
}

#[derive(Debug)]
pub enum VideoOrPlaylist {
    Video(VideoInfo),
    Playlist(PlaylistInfo),
}

impl VideoOrPlaylist {
    pub fn is_empty(&self) -> bool {
        self.videos().is_empty()
    }

    pub fn title(&self) -> &str {
        match self {
            VideoOrPlaylist::Video(v) => &v.title,
            VideoOrPlaylist::Playlist(p) => &p.title,
        }
    }

    pub fn thumbnail(&self) -> Option<&str> {
        match self {
            VideoOrPlaylist::Video(v) => v.thumbnail.as_ref().map(|x| x.as_str()),
            VideoOrPlaylist::Playlist(p) => p.thumbnail.as_ref().map(|x| x.as_str()),
        }
    }

    pub fn videos(&self) -> &[VideoInfo] {
        match self {
            VideoOrPlaylist::Video(v) => slice::from_ref(v),
            VideoOrPlaylist::Playlist(p) => &p.videos,
        }
    }
}
