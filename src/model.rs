//
// model.rs
// Copyright (C) 2019 gmg137 <gmg137@live.com>
// Distributed under terms of the GPLv3 license.
//
use anyhow::{anyhow, Ok, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fmt;

#[allow(unused)]
pub fn to_lyric(json: String) -> Result<Vec<String>> {
    let value = serde_json::from_str::<Value>(&json)?;
    if value
        .get("code")
        .ok_or_else(|| anyhow!("none"))?
        .eq(&json!(200i32))
    {
        let mut vec: Vec<String> = Vec::new();
        let lyric = value
            .get("lrc")
            .ok_or_else(|| anyhow!("none"))?
            .get("lyric")
            .ok_or_else(|| anyhow!("none"))?
            .as_str()
            .ok_or_else(|| anyhow!("none"))?
            .to_owned();
        vec = lyric
            .split('\n')
            .collect::<Vec<&str>>()
            .iter()
            .map(|s| (*s).to_string())
            .filter(|s| !s.is_empty())
            .collect::<Vec<String>>();
        return Ok(vec);
    }
    Err(anyhow!("none"))
}

/// 歌手信息
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SingerInfo {
    /// 歌手 id
    pub id: u64,
    /// 歌手姓名
    pub name: String,
    /// 歌手照片
    pub pic_url: String,
}

#[allow(unused)]
pub fn to_singer_info(json: String) -> Result<Vec<SingerInfo>> {
    let value = serde_json::from_str::<Value>(&json)?;
    if value
        .get("code")
        .ok_or_else(|| anyhow!("none"))?
        .eq(&json!(200i32))
    {
        let mut vec: Vec<SingerInfo> = Vec::new();
        let array = value
            .get("result")
            .ok_or_else(|| anyhow!("none"))?
            .get("artists")
            .ok_or_else(|| anyhow!("none"))?
            .as_array()
            .ok_or_else(|| anyhow!("none"))?;
        for v in array.iter() {
            let mut pic_url = v
                .get("img1v1Url")
                .unwrap_or(&json!(""))
                .as_str()
                .unwrap_or("")
                .to_owned();
            if pic_url.ends_with("5639395138885805.jpg") {
                pic_url = "".to_owned();
            }
            vec.push(SingerInfo {
                id: v
                    .get("id")
                    .ok_or_else(|| anyhow!("none"))?
                    .as_u64()
                    .ok_or_else(|| anyhow!("none"))? as u64,
                name: v
                    .get("name")
                    .ok_or_else(|| anyhow!("none"))?
                    .as_str()
                    .ok_or_else(|| anyhow!("none"))?
                    .to_owned(),
                pic_url,
            });
        }
        return Ok(vec);
    }
    Err(anyhow!("none"))
}

/// 歌曲 URL
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SongUrl {
    /// 歌曲 id
    pub id: u64,
    /// 歌曲 URL
    pub url: String,
    /// 码率
    pub rate: u32,
}

#[allow(unused)]
pub fn to_song_url(json: String) -> Result<Vec<SongUrl>> {
    let value = serde_json::from_str::<Value>(&json)?;
    if value.get("code").ok_or_else(|| anyhow!("none"))?.eq(&200) {
        let mut vec: Vec<SongUrl> = Vec::new();
        let array = value
            .get("data")
            .ok_or_else(|| anyhow!("none"))?
            .as_array()
            .ok_or_else(|| anyhow!("none"))?;
        for v in array.iter() {
            let url = v
                .get("url")
                .unwrap_or(&json!(""))
                .as_str()
                .unwrap_or("")
                .to_owned();
            if !url.is_empty() {
                vec.push(SongUrl {
                    id: v
                        .get("id")
                        .ok_or_else(|| anyhow!("none"))?
                        .as_u64()
                        .ok_or_else(|| anyhow!("none"))? as u64,
                    url,
                    rate: v
                        .get("br")
                        .ok_or_else(|| anyhow!("none"))?
                        .as_u64()
                        .ok_or_else(|| anyhow!("none"))? as u32,
                });
            }
        }
        return Ok(vec);
    }
    Err(anyhow!("none"))
}

/// 歌曲信息
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SongInfo {
    /// 歌曲 id
    pub id: u64,
    /// 歌名
    pub name: String,
    /// 歌手
    pub singer: String,
    /// 专辑
    pub album: String,
    /// 专辑 ID
    pub album_id: u64,
    /// 封面图
    pub pic_url: String,
    /// 歌曲时长
    pub duration: String,
    /// 歌曲链接
    pub song_url: String,
}

impl PartialEq for SongInfo {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

/// parse: 解析方式
#[allow(unused)]
pub fn to_song_info(json: String, parse: Parse) -> Result<Vec<SongInfo>> {
    let value = serde_json::from_str::<Value>(&json)?;
    if value.get("code").ok_or_else(|| anyhow!("none"))?.eq(&200) {
        let mut vec: Vec<SongInfo> = Vec::new();
        let list = json!([]);
        match parse {
            Parse::Usl => {
                let mut array = value
                    .get("songs")
                    .unwrap_or(&list)
                    .as_array()
                    .ok_or_else(|| anyhow!("none"))?;
                if array.is_empty() {
                    array = value
                        .get("playlist")
                        .ok_or_else(|| anyhow!("none"))?
                        .get("tracks")
                        .ok_or_else(|| anyhow!("none"))?
                        .as_array()
                        .ok_or_else(|| anyhow!("none"))?;
                }
                for v in array.iter() {
                    let duration = v
                        .get("dt")
                        .ok_or_else(|| anyhow!("none"))?
                        .as_u64()
                        .ok_or_else(|| anyhow!("none"))? as u32;
                    vec.push(SongInfo {
                        id: v
                            .get("id")
                            .ok_or_else(|| anyhow!("none"))?
                            .as_u64()
                            .ok_or_else(|| anyhow!("none"))?,
                        name: v
                            .get("name")
                            .ok_or_else(|| anyhow!("none"))?
                            .as_str()
                            .ok_or_else(|| anyhow!("none"))?
                            .to_owned(),
                        singer: v
                            .get("ar")
                            .ok_or_else(|| anyhow!("none"))?
                            .get(0)
                            .ok_or_else(|| anyhow!("none"))?
                            .get("name")
                            .unwrap_or(&json!("未知"))
                            .as_str()
                            .unwrap_or("未知")
                            .to_owned(),
                        album: v
                            .get("al")
                            .ok_or_else(|| anyhow!("none"))?
                            .get("name")
                            .unwrap_or(&json!("未知"))
                            .as_str()
                            .unwrap_or("未知")
                            .to_owned(),
                        album_id: v
                            .get("al")
                            .ok_or_else(|| anyhow!("none"))?
                            .get("id")
                            .ok_or_else(|| anyhow!("none"))?
                            .as_u64()
                            .ok_or_else(|| anyhow!("none"))?,
                        pic_url: v
                            .get("al")
                            .ok_or_else(|| anyhow!("none"))?
                            .get("picUrl")
                            .unwrap_or(&json!(""))
                            .as_str()
                            .unwrap_or("")
                            .to_owned(),
                        duration: format!(
                            "{:0>2}:{:0>2}",
                            duration / 1000 / 60,
                            duration / 1000 % 60
                        ),
                        song_url: String::new(),
                    });
                }
            }
            Parse::Ucd => {
                let array = value
                    .get("data")
                    .ok_or_else(|| anyhow!("none"))?
                    .as_array()
                    .ok_or_else(|| anyhow!("none"))?;
                for v in array.iter() {
                    let duration = v
                        .get("simpleSong")
                        .ok_or_else(|| anyhow!("none"))?
                        .get("dt")
                        .ok_or_else(|| anyhow!("none"))?
                        .as_u64()
                        .ok_or_else(|| anyhow!("none"))? as u32;
                    vec.push(SongInfo {
                        id: v
                            .get("songId")
                            .ok_or_else(|| anyhow!("none"))?
                            .as_u64()
                            .ok_or_else(|| anyhow!("none"))? as u64,
                        name: v
                            .get("songName")
                            .ok_or_else(|| anyhow!("none"))?
                            .as_str()
                            .ok_or_else(|| anyhow!("none"))?
                            .to_owned(),
                        singer: v
                            .get("artist")
                            .unwrap_or(&json!("未知"))
                            .as_str()
                            .unwrap_or("未知")
                            .to_owned(),
                        album: v
                            .get("album")
                            .unwrap_or(&json!("未知"))
                            .as_str()
                            .unwrap_or("未知")
                            .to_owned(),
                        album_id: 0,
                        pic_url: String::new(),
                        duration: format!(
                            "{:0>2}:{:0>2}",
                            duration / 1000 / 60,
                            duration / 1000 % 60
                        ),
                        song_url: String::new(),
                    });
                }
            }
            Parse::Rmd => {
                let array = value
                    .get("data")
                    .ok_or_else(|| anyhow!("none"))?
                    .as_array()
                    .ok_or_else(|| anyhow!("none"))?;
                for v in array.iter() {
                    let duration = v
                        .get("duration")
                        .ok_or_else(|| anyhow!("none"))?
                        .as_u64()
                        .ok_or_else(|| anyhow!("none"))? as u32;
                    vec.push(SongInfo {
                        id: v
                            .get("id")
                            .ok_or_else(|| anyhow!("none"))?
                            .as_u64()
                            .ok_or_else(|| anyhow!("none"))? as u64,
                        name: v
                            .get("name")
                            .ok_or_else(|| anyhow!("none"))?
                            .as_str()
                            .ok_or_else(|| anyhow!("none"))?
                            .to_owned(),
                        singer: v
                            .get("artists")
                            .ok_or_else(|| anyhow!("none"))?
                            .get(0)
                            .ok_or_else(|| anyhow!("none"))?
                            .get("name")
                            .unwrap_or(&json!("未知"))
                            .as_str()
                            .unwrap_or("未知")
                            .to_owned(),
                        album: v
                            .get("album")
                            .ok_or_else(|| anyhow!("none"))?
                            .get("name")
                            .unwrap_or(&json!("未知"))
                            .as_str()
                            .unwrap_or("未知")
                            .to_owned(),
                        album_id: v
                            .get("album")
                            .ok_or_else(|| anyhow!("none"))?
                            .get("id")
                            .ok_or_else(|| anyhow!("none"))?
                            .as_u64()
                            .ok_or_else(|| anyhow!("none"))?,
                        pic_url: v
                            .get("album")
                            .ok_or_else(|| anyhow!("none"))?
                            .get("picUrl")
                            .unwrap_or(&json!(""))
                            .as_str()
                            .unwrap_or("")
                            .to_owned(),
                        duration: format!(
                            "{:0>2}:{:0>2}",
                            duration / 1000 / 60,
                            duration / 1000 % 60
                        ),
                        song_url: String::new(),
                    });
                }
            }
            Parse::Rmds => {
                let array = value
                    .get("data")
                    .ok_or_else(|| anyhow!("none"))?
                    .as_object()
                    .ok_or_else(|| anyhow!("none"))?
                    .get("dailySongs")
                    .ok_or_else(|| anyhow!("none"))?
                    .as_array()
                    .ok_or_else(|| anyhow!("none"))?;
                for v in array.iter() {
                    let duration = v
                        .get("duration")
                        .ok_or_else(|| anyhow!("none"))?
                        .as_u64()
                        .ok_or_else(|| anyhow!("none"))? as u32;
                    vec.push(SongInfo {
                        id: v
                            .get("id")
                            .ok_or_else(|| anyhow!("none"))?
                            .as_u64()
                            .ok_or_else(|| anyhow!("none"))? as u64,
                        name: v
                            .get("name")
                            .ok_or_else(|| anyhow!("none"))?
                            .as_str()
                            .ok_or_else(|| anyhow!("none"))?
                            .to_owned(),
                        singer: v
                            .get("artists")
                            .ok_or_else(|| anyhow!("none"))?
                            .get(0)
                            .ok_or_else(|| anyhow!("none"))?
                            .get("name")
                            .unwrap_or(&json!("未知"))
                            .as_str()
                            .unwrap_or("未知")
                            .to_owned(),
                        album: v
                            .get("album")
                            .ok_or_else(|| anyhow!("none"))?
                            .get("name")
                            .unwrap_or(&json!("未知"))
                            .as_str()
                            .unwrap_or("未知")
                            .to_owned(),
                        album_id: v
                            .get("album")
                            .ok_or_else(|| anyhow!("none"))?
                            .get("id")
                            .ok_or_else(|| anyhow!("none"))?
                            .as_u64()
                            .ok_or_else(|| anyhow!("none"))?,
                        pic_url: v
                            .get("album")
                            .ok_or_else(|| anyhow!("none"))?
                            .get("picUrl")
                            .unwrap_or(&json!(""))
                            .as_str()
                            .unwrap_or("")
                            .to_owned(),
                        duration: format!(
                            "{:0>2}:{:0>2}",
                            duration / 1000 / 60,
                            duration / 1000 % 60
                        ),
                        song_url: String::new(),
                    });
                }
            }
            Parse::Search => {
                let array = value
                    .get("result")
                    .ok_or_else(|| anyhow!("none"))?
                    .as_object()
                    .ok_or_else(|| anyhow!("none"))?
                    .get("songs")
                    .ok_or_else(|| anyhow!("none"))?
                    .as_array()
                    .ok_or_else(|| anyhow!("none"))?;
                for v in array.iter() {
                    let duration = v
                        .get("duration")
                        .ok_or_else(|| anyhow!("none"))?
                        .as_u64()
                        .ok_or_else(|| anyhow!("none"))? as u32;
                    vec.push(SongInfo {
                        id: v
                            .get("id")
                            .ok_or_else(|| anyhow!("none"))?
                            .as_u64()
                            .ok_or_else(|| anyhow!("none"))? as u64,
                        name: v
                            .get("name")
                            .ok_or_else(|| anyhow!("none"))?
                            .as_str()
                            .ok_or_else(|| anyhow!("none"))?
                            .to_owned(),
                        singer: v
                            .get("artists")
                            .ok_or_else(|| anyhow!("none"))?
                            .get(0)
                            .ok_or_else(|| anyhow!("none"))?
                            .get("name")
                            .unwrap_or(&json!("未知"))
                            .as_str()
                            .unwrap_or("未知")
                            .to_owned(),
                        album: v
                            .get("album")
                            .ok_or_else(|| anyhow!("none"))?
                            .get("name")
                            .unwrap_or(&json!("未知"))
                            .as_str()
                            .unwrap_or("未知")
                            .to_owned(),
                        album_id: v
                            .get("album")
                            .ok_or_else(|| anyhow!("none"))?
                            .get("id")
                            .ok_or_else(|| anyhow!("none"))?
                            .as_u64()
                            .ok_or_else(|| anyhow!("none"))?,
                        pic_url: v
                            .get("album")
                            .ok_or_else(|| anyhow!("none"))?
                            .get("picUrl")
                            .unwrap_or(&json!(""))
                            .as_str()
                            .unwrap_or("")
                            .to_owned(),
                        duration: format!(
                            "{:0>2}:{:0>2}",
                            duration / 1000 / 60,
                            duration / 1000 % 60
                        ),
                        song_url: String::new(),
                    });
                }
            }
            Parse::Album => {
                let array = value
                    .get("songs")
                    .ok_or_else(|| anyhow!("none"))?
                    .as_array()
                    .ok_or_else(|| anyhow!("none"))?;
                for v in array.iter() {
                    let duration = v
                        .get("dt")
                        .ok_or_else(|| anyhow!("none"))?
                        .as_u64()
                        .ok_or_else(|| anyhow!("none"))? as u32;
                    vec.push(SongInfo {
                        id: v
                            .get("id")
                            .ok_or_else(|| anyhow!("none"))?
                            .as_u64()
                            .ok_or_else(|| anyhow!("none"))? as u64,
                        name: v
                            .get("name")
                            .ok_or_else(|| anyhow!("none"))?
                            .as_str()
                            .ok_or_else(|| anyhow!("none"))?
                            .to_owned(),
                        singer: v
                            .get("ar")
                            .ok_or_else(|| anyhow!("none"))?
                            .get(0)
                            .ok_or_else(|| anyhow!("none"))?
                            .get("name")
                            .unwrap_or(&json!("未知"))
                            .as_str()
                            .unwrap_or("未知")
                            .to_owned(),
                        album: value
                            .get("album")
                            .ok_or_else(|| anyhow!("none"))?
                            .get("name")
                            .unwrap_or(&json!("未知"))
                            .as_str()
                            .unwrap_or("未知")
                            .to_owned(),
                        album_id: value
                            .get("album")
                            .ok_or_else(|| anyhow!("none"))?
                            .get("id")
                            .ok_or_else(|| anyhow!("none"))?
                            .as_u64()
                            .ok_or_else(|| anyhow!("none"))?,
                        pic_url: value
                            .get("album")
                            .ok_or_else(|| anyhow!("none"))?
                            .get("picUrl")
                            .unwrap_or(&json!(""))
                            .as_str()
                            .unwrap_or("")
                            .to_owned(),
                        duration: format!(
                            "{:0>2}:{:0>2}",
                            duration / 1000 / 60,
                            duration / 1000 % 60
                        ),
                        song_url: String::new(),
                    });
                }
            }
            Parse::Singer => {
                let array = value
                    .get("hotSongs")
                    .ok_or_else(|| anyhow!("none"))?
                    .as_array()
                    .ok_or_else(|| anyhow!("none"))?;
                let ar = value
                    .get("artist")
                    .ok_or_else(|| anyhow!("none"))?
                    .get("name")
                    .ok_or_else(|| anyhow!("none"))?
                    .as_str()
                    .ok_or_else(|| anyhow!("none"))?
                    .to_owned();
                for v in array.iter() {
                    let duration = v
                        .get("dt")
                        .ok_or_else(|| anyhow!("none"))?
                        .as_u64()
                        .ok_or_else(|| anyhow!("none"))? as u32;
                    vec.push(SongInfo {
                        id: v
                            .get("id")
                            .ok_or_else(|| anyhow!("none"))?
                            .as_u64()
                            .ok_or_else(|| anyhow!("none"))? as u64,
                        name: v
                            .get("name")
                            .ok_or_else(|| anyhow!("none"))?
                            .as_str()
                            .ok_or_else(|| anyhow!("none"))?
                            .to_owned(),
                        singer: ar.to_owned(),
                        album: v
                            .get("al")
                            .ok_or_else(|| anyhow!("none"))?
                            .get("name")
                            .unwrap_or(&json!("未知"))
                            .as_str()
                            .unwrap_or("未知")
                            .to_owned(),
                        album_id: v
                            .get("al")
                            .ok_or_else(|| anyhow!("none"))?
                            .get("id")
                            .ok_or_else(|| anyhow!("none"))?
                            .as_u64()
                            .ok_or_else(|| anyhow!("none"))?,
                        pic_url: "".to_owned(),
                        duration: format!(
                            "{:0>2}:{:0>2}",
                            duration / 1000 / 60,
                            duration / 1000 % 60
                        ),
                        song_url: String::new(),
                    });
                }
            }
            _ => {}
        }
        return Ok(vec);
    }
    Err(anyhow!("none"))
}

/// 歌单信息
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SongList {
    /// 歌单 id
    pub id: u64,
    /// 歌单名
    pub name: String,
    /// 歌单封面
    pub cover_img_url: String,
}

/// parse: 解析方式
#[allow(unused)]
pub fn to_song_list(json: String, parse: Parse) -> Result<Vec<SongList>> {
    let value = serde_json::from_str::<Value>(&json)?;
    if value.get("code").ok_or_else(|| anyhow!("none"))?.eq(&200) {
        let mut vec: Vec<SongList> = Vec::new();
        match parse {
            Parse::Usl => {
                let array = value
                    .get("playlist")
                    .ok_or_else(|| anyhow!("none"))?
                    .as_array()
                    .ok_or_else(|| anyhow!("none"))?;
                for v in array.iter() {
                    vec.push(SongList {
                        id: v
                            .get("id")
                            .ok_or_else(|| anyhow!("none"))?
                            .as_u64()
                            .ok_or_else(|| anyhow!("none"))? as u64,
                        name: v
                            .get("name")
                            .ok_or_else(|| anyhow!("none"))?
                            .as_str()
                            .ok_or_else(|| anyhow!("none"))?
                            .to_owned(),
                        cover_img_url: v
                            .get("coverImgUrl")
                            .ok_or_else(|| anyhow!("none"))?
                            .as_str()
                            .ok_or_else(|| anyhow!("none"))?
                            .to_owned(),
                    });
                }
            }
            Parse::Rmd => {
                let array = value
                    .get("recommend")
                    .ok_or_else(|| anyhow!("none"))?
                    .as_array()
                    .ok_or_else(|| anyhow!("none"))?;
                for v in array.iter() {
                    vec.push(SongList {
                        id: v
                            .get("id")
                            .ok_or_else(|| anyhow!("none"))?
                            .as_u64()
                            .ok_or_else(|| anyhow!("none"))? as u64,
                        name: v
                            .get("name")
                            .ok_or_else(|| anyhow!("none"))?
                            .as_str()
                            .ok_or_else(|| anyhow!("none"))?
                            .to_owned(),
                        cover_img_url: v
                            .get("picUrl")
                            .unwrap_or(&json!(""))
                            .as_str()
                            .unwrap_or("")
                            .to_owned(),
                    });
                }
            }
            Parse::Album => {
                let array = value
                    .get("albums")
                    .ok_or_else(|| anyhow!("none"))?
                    .as_array()
                    .ok_or_else(|| anyhow!("none"))?;
                for v in array.iter() {
                    vec.push(SongList {
                        id: v
                            .get("id")
                            .ok_or_else(|| anyhow!("none"))?
                            .as_u64()
                            .ok_or_else(|| anyhow!("none"))? as u64,
                        name: v
                            .get("name")
                            .ok_or_else(|| anyhow!("none"))?
                            .as_str()
                            .ok_or_else(|| anyhow!("none"))?
                            .to_owned(),
                        cover_img_url: v
                            .get("picUrl")
                            .unwrap_or(&json!(""))
                            .as_str()
                            .unwrap_or("")
                            .to_owned(),
                    });
                }
            }
            Parse::Top => {
                let array = value
                    .get("playlists")
                    .ok_or_else(|| anyhow!("none"))?
                    .as_array()
                    .ok_or_else(|| anyhow!("none"))?;
                for v in array.iter() {
                    vec.push(SongList {
                        id: v
                            .get("id")
                            .ok_or_else(|| anyhow!("none"))?
                            .as_u64()
                            .ok_or_else(|| anyhow!("none"))? as u64,
                        name: v
                            .get("name")
                            .ok_or_else(|| anyhow!("none"))?
                            .as_str()
                            .ok_or_else(|| anyhow!("none"))?
                            .to_owned(),
                        cover_img_url: v
                            .get("coverImgUrl")
                            .ok_or_else(|| anyhow!("none"))?
                            .as_str()
                            .ok_or_else(|| anyhow!("none"))?
                            .to_owned(),
                    });
                }
            }
            Parse::Search => {
                let array = value
                    .get("result")
                    .ok_or_else(|| anyhow!("none"))?
                    .get("playlists")
                    .ok_or_else(|| anyhow!("none"))?
                    .as_array()
                    .ok_or_else(|| anyhow!("none"))?;
                for v in array.iter() {
                    vec.push(SongList {
                        id: v
                            .get("id")
                            .ok_or_else(|| anyhow!("none"))?
                            .as_u64()
                            .ok_or_else(|| anyhow!("none"))? as u64,
                        name: v
                            .get("name")
                            .ok_or_else(|| anyhow!("none"))?
                            .as_str()
                            .ok_or_else(|| anyhow!("none"))?
                            .to_owned(),
                        cover_img_url: v
                            .get("coverImgUrl")
                            .ok_or_else(|| anyhow!("none"))?
                            .as_str()
                            .ok_or_else(|| anyhow!("none"))?
                            .to_owned(),
                    });
                }
            }
            Parse::SearchAlbum => {
                let array = value
                    .get("result")
                    .ok_or_else(|| anyhow!("none"))?
                    .get("albums")
                    .ok_or_else(|| anyhow!("none"))?
                    .as_array()
                    .ok_or_else(|| anyhow!("none"))?;
                for v in array.iter() {
                    vec.push(SongList {
                        id: v
                            .get("id")
                            .ok_or_else(|| anyhow!("none"))?
                            .as_u64()
                            .ok_or_else(|| anyhow!("none"))? as u64,
                        name: v
                            .get("name")
                            .ok_or_else(|| anyhow!("none"))?
                            .as_str()
                            .ok_or_else(|| anyhow!("none"))?
                            .to_owned(),
                        cover_img_url: v
                            .get("picUrl")
                            .ok_or_else(|| anyhow!("none"))?
                            .as_str()
                            .ok_or_else(|| anyhow!("none"))?
                            .to_owned(),
                    });
                }
            }
            Parse::LikeAlbum => {
                let array = value
                    .get("data")
                    .ok_or_else(|| anyhow!("none"))?
                    .as_array()
                    .ok_or_else(|| anyhow!("none"))?;
                for v in array.iter() {
                    vec.push(SongList {
                        id: v
                            .get("id")
                            .ok_or_else(|| anyhow!("none"))?
                            .as_u64()
                            .ok_or_else(|| anyhow!("none"))? as u64,
                        name: v
                            .get("name")
                            .ok_or_else(|| anyhow!("none"))?
                            .as_str()
                            .ok_or_else(|| anyhow!("none"))?
                            .to_owned(),
                        cover_img_url: v
                            .get("picUrl")
                            .ok_or_else(|| anyhow!("none"))?
                            .as_str()
                            .ok_or_else(|| anyhow!("none"))?
                            .to_owned(),
                    });
                }
            }
            _ => {}
        }
        return Ok(vec);
    }
    Err(anyhow!("none"))
}

/// 消息
#[derive(Debug, Deserialize, Serialize)]
pub struct Msg {
    pub code: i32,
    pub msg: String,
}

#[allow(unused)]
pub fn to_msg(json: String) -> Result<Msg> {
    let value = serde_json::from_str::<Value>(&json)?;
    let code = value
        .get("code")
        .ok_or_else(|| anyhow!("none"))?
        .as_i64()
        .ok_or_else(|| anyhow!("none"))? as i32;
    if code.eq(&200) {
        return Ok(Msg {
            code: 200,
            msg: "".to_owned(),
        });
    }
    let msg = value
        .get("msg")
        .ok_or_else(|| anyhow!("none"))?
        .as_str()
        .ok_or_else(|| anyhow!("none"))?
        .to_owned();
    Ok(Msg { code, msg })
}

#[allow(unused)]
pub fn to_message(json: String) -> Result<Msg> {
    let value = serde_json::from_str::<Value>(&json)?;
    let code = value
        .get("code")
        .ok_or_else(|| anyhow!("none"))?
        .as_i64()
        .ok_or_else(|| anyhow!("none"))? as i32;
    if code.eq(&200) {
        return Ok(Msg {
            code: 200,
            msg: "".to_owned(),
        });
    }
    let msg = value
        .get("message")
        .ok_or_else(|| anyhow!("none"))?
        .as_str()
        .ok_or_else(|| anyhow!("none"))?
        .to_owned();
    Ok(Msg { code, msg })
}

/// 登录信息
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LoginInfo {
    /// 登录状态码
    pub code: i32,
    /// 用户 id
    pub uid: u64,
    /// 用户昵称
    pub nickname: String,
    /// 用户头像
    pub avatar_url: String,
    /// 状态消息
    pub msg: String,
}

#[allow(unused)]
pub fn to_login_info(json: String) -> Result<LoginInfo> {
    let value = serde_json::from_str::<Value>(&json)?;
    let code = value
        .get("code")
        .ok_or_else(|| anyhow!("none"))?
        .as_i64()
        .ok_or_else(|| anyhow!("none"))? as i32;
    if code.eq(&200) {
        let profile = value
            .get("profile")
            .ok_or_else(|| anyhow!("none"))?
            .as_object()
            .ok_or_else(|| anyhow!("none"))?;
        return Ok(LoginInfo {
            code,
            uid: profile
                .get("userId")
                .ok_or_else(|| anyhow!("none"))?
                .as_u64()
                .ok_or_else(|| anyhow!("none"))? as u64,
            nickname: profile
                .get("nickname")
                .ok_or_else(|| anyhow!("none"))?
                .as_str()
                .ok_or_else(|| anyhow!("none"))?
                .to_owned(),
            avatar_url: profile
                .get("avatarUrl")
                .ok_or_else(|| anyhow!("none"))?
                .as_str()
                .ok_or_else(|| anyhow!("none"))?
                .to_owned(),
            msg: "".to_owned(),
        });
    }
    let msg = value
        .get("msg")
        .ok_or_else(|| anyhow!("none"))?
        .as_str()
        .ok_or_else(|| anyhow!("none"))?
        .to_owned();
    Ok(LoginInfo {
        code,
        uid: 0,
        nickname: "".to_owned(),
        avatar_url: "".to_owned(),
        msg,
    })
}

/// 轮播信息
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct BannersInfo {
    /// 轮播图
    pub pic: String,
    /// 歌曲 id
    pub id: u64,
    /// 歌名
    pub name: String,
    /// 歌手
    pub singer: String,
    /// 专辑
    pub album: String,
    /// 专辑封面
    pub pic_url: String,
    /// 时长
    pub duration: String,
}

#[allow(unused)]
pub fn to_banners_info(json: String) -> Result<Vec<BannersInfo>> {
    let value = serde_json::from_str::<Value>(&json)?;
    if value
        .get("code")
        .ok_or_else(|| anyhow!("none"))?
        .eq(&200i32)
    {
        let array = value
            .get("data")
            .ok_or_else(|| anyhow!("none"))?
            .get("blocks")
            .ok_or_else(|| anyhow!("none"))?
            .as_array()
            .ok_or_else(|| anyhow!("none"))?;
        for v in array.iter() {
            let show_type = v.get("showType").ok_or_else(|| anyhow!("none"))?;
            if show_type.eq("BANNER") {
                let mut vec: Vec<BannersInfo> = Vec::new();
                let banners = v
                    .get("extInfo")
                    .ok_or_else(|| anyhow!("none"))?
                    .get("banners")
                    .ok_or_else(|| anyhow!("none"))?
                    .as_array()
                    .ok_or_else(|| anyhow!("none"))?;
                for v in banners.iter() {
                    if v.get("typeTitle")
                        .ok_or_else(|| anyhow!("none"))?
                        .as_str()
                        .ok_or_else(|| anyhow!("none"))?
                        .eq("新歌首发")
                    {
                        if let Some(song) = v.get("song") {
                            if song.is_null() {
                                continue;
                            }
                            let duration = song
                                .get("dt")
                                .ok_or_else(|| anyhow!("none"))?
                                .as_u64()
                                .ok_or_else(|| anyhow!("none"))?;
                            vec.push(BannersInfo {
                                pic: v
                                    .get("pic")
                                    .ok_or_else(|| anyhow!("none"))?
                                    .as_str()
                                    .ok_or_else(|| anyhow!("none"))?
                                    .to_owned(),
                                name: song
                                    .get("name")
                                    .ok_or_else(|| anyhow!("none"))?
                                    .as_str()
                                    .ok_or_else(|| anyhow!("none"))?
                                    .to_owned(),
                                id: song
                                    .get("id")
                                    .ok_or_else(|| anyhow!("none"))?
                                    .as_u64()
                                    .ok_or_else(|| anyhow!("none"))?,
                                singer: song
                                    .get("ar")
                                    .ok_or_else(|| anyhow!("none"))?
                                    .as_array()
                                    .ok_or_else(|| anyhow!("none"))?
                                    .get(0)
                                    .ok_or_else(|| anyhow!("none"))?
                                    .get("name")
                                    .ok_or_else(|| anyhow!("none"))?
                                    .as_str()
                                    .ok_or_else(|| anyhow!("none"))?
                                    .to_owned(),
                                album: song
                                    .get("al")
                                    .ok_or_else(|| anyhow!("none"))?
                                    .get("name")
                                    .ok_or_else(|| anyhow!("none"))?
                                    .as_str()
                                    .ok_or_else(|| anyhow!("none"))?
                                    .to_owned(),
                                pic_url: song
                                    .get("al")
                                    .ok_or_else(|| anyhow!("none"))?
                                    .get("picUrl")
                                    .ok_or_else(|| anyhow!("none"))?
                                    .as_str()
                                    .ok_or_else(|| anyhow!("none"))?
                                    .to_owned(),
                                duration: format!(
                                    "{:0>2}:{:0>2}",
                                    duration / 1000 / 60,
                                    duration / 1000 % 60
                                ),
                            });
                        }
                    };
                }
                return Ok(vec);
                break;
            }
        }
    }
    Err(anyhow!("none"))
}

#[allow(unused)]
pub fn to_captcha(json: String) -> Result<()> {
    let value = serde_json::from_str::<Value>(&json)?;
    let code = value
        .get("code")
        .ok_or_else(|| anyhow!("none"))?
        .as_i64()
        .ok_or_else(|| anyhow!("none"))? as i32;
    if code.eq(&200) {
        return Ok(());
    }
    let data = value
        .get("data")
        .ok_or_else(|| anyhow!("none"))?
        .as_bool()
        .ok_or_else(|| anyhow!("none"))?
        .to_owned();
    if data {
        return Ok(());
    }
    Err(anyhow!("get captcha err!"))
}

#[allow(unused)]
pub fn to_unikey(json: String) -> Result<String> {
    let value = serde_json::from_str::<Value>(&json)?;
    let code = value
        .get("code")
        .ok_or_else(|| anyhow!("none"))?
        .as_i64()
        .ok_or_else(|| anyhow!("none"))? as i32;
    if code.eq(&200) {
        let unikey = value
            .get("unikey")
            .ok_or_else(|| anyhow!("none"))?
            .as_str()
            .ok_or_else(|| anyhow!("none"))?
            .to_owned();
        return Ok(unikey);
    }
    Err(anyhow!("get unikey err!"))
}

/// 轮播信息
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TopList {
    /// 榜单 id
    pub id: u64,
    /// 榜单名
    pub name: String,
    /// 更新时间
    pub update: String,
    /// 榜单简介
    pub description: String,
    /// 榜单封面
    pub cover: String,
}

#[allow(unused)]
pub fn to_toplist(json: String) -> Result<Vec<TopList>> {
    let value = serde_json::from_str::<Value>(&json)?;
    let code = value
        .get("code")
        .ok_or_else(|| anyhow!("none"))?
        .as_i64()
        .ok_or_else(|| anyhow!("none"))? as i32;
    if code.eq(&200) {
        let list = value
            .get("list")
            .ok_or_else(|| anyhow!("none"))?
            .as_array()
            .ok_or_else(|| anyhow!("none"))?
            .to_owned();
        let mut toplist = Vec::new();
        for t in list {
            toplist.push(TopList {
                id: t
                    .get("id")
                    .ok_or_else(|| anyhow!("none"))?
                    .as_u64()
                    .ok_or_else(|| anyhow!("none"))?,
                name: t
                    .get("name")
                    .ok_or_else(|| anyhow!("none"))?
                    .as_str()
                    .ok_or_else(|| anyhow!("none"))?
                    .to_owned(),
                update: t
                    .get("updateFrequency")
                    .unwrap_or(&json!(""))
                    .as_str()
                    .unwrap_or("")
                    .to_owned(),
                description: t
                    .get("description")
                    .unwrap_or(&json!(""))
                    .as_str()
                    .unwrap_or("")
                    .to_owned(),
                cover: t
                    .get("coverImgUrl")
                    .unwrap_or(&json!(""))
                    .as_str()
                    .unwrap_or("")
                    .to_owned(),
            });
        }
        return Ok(toplist);
    }
    Err(anyhow!("get toplist err!"))
}

/// 请求方式
#[allow(unused)]
#[derive(Debug)]
pub enum Method {
    Post,
    Get,
}

/// 解析方式
/// USL: 用户
/// UCD: 云盘
/// RMD: 推荐
/// RMDS: 推荐歌曲
/// SEARCH: 搜索
/// SD: 单曲详情
/// ALBUM: 专辑
/// LikeAlbum: 收藏的专辑
/// TOP: 热门
/// Singer: 歌手热门单曲
#[allow(unused)]
#[derive(Debug, Clone)]
pub enum Parse {
    Usl,
    Ucd,
    Rmd,
    Rmds,
    Search,
    SearchAlbum,
    LikeAlbum,
    Sd,
    Album,
    Top,
    Singer,
}

/// 客户端类型
#[allow(unused)]
#[derive(Debug, Clone)]
pub enum ClientType {
    Pc,
    Android,
    Iphone,
    Ipad,
}

impl fmt::Display for ClientType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = match self {
            Self::Pc => "pc".to_owned(),
            Self::Android => "android".to_owned(),
            Self::Iphone => "iphone".to_owned(),
            Self::Ipad => "ipad".to_owned(),
        };
        write!(f, "{s}")
    }
}
