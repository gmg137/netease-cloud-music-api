//
// mod.rs
// Copyright (C) 2019 gmg137 <gmg137@live.com>
// Distributed under terms of the GPLv3 license.
//
mod encrypt;
pub(crate) mod model;
use anyhow::{anyhow, Result};
use encrypt::Crypto;
pub use isahc::cookies::CookieJar;
use isahc::{prelude::*, *};
use lazy_static::lazy_static;
pub use model::*;
use regex::Regex;
use std::{cell::RefCell, collections::HashMap, path::PathBuf, time::Duration};
use urlqstring::QueryParams;

lazy_static! {
    static ref _CSRF: Regex = Regex::new(r"_csrf=(?P<csrf>[^(;|$)]+)").unwrap();
}

static BASE_URL: &str = "https://music.163.com";

const LINUX_USER_AGNET: &str =
    "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/60.0.3112.90 Safari/537.36";

const USER_AGENT_LIST: [&str; 14] = [
    "Mozilla/5.0 (iPhone; CPU iPhone OS 9_1 like Mac OS X) AppleWebKit/601.1.46 (KHTML, like Gecko) Version/9.0 Mobile/13B143 Safari/601.1",
    "Mozilla/5.0 (iPhone; CPU iPhone OS 9_1 like Mac OS X) AppleWebKit/601.1.46 (KHTML, like Gecko) Version/9.0 Mobile/13B143 Safari/601.1",
    "Mozilla/5.0 (Linux; Android 5.0; SM-G900P Build/LRX21T) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/59.0.3071.115 Mobile Safari/537.36",
    "Mozilla/5.0 (Linux; Android 6.0; Nexus 5 Build/MRA58N) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/59.0.3071.115 Mobile Safari/537.36",
    "Mozilla/5.0 (Linux; Android 5.1.1; Nexus 6 Build/LYZ28E) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/59.0.3071.115 Mobile Safari/537.36",
    "Mozilla/5.0 (iPhone; CPU iPhone OS 10_3_2 like Mac OS X) AppleWebKit/603.2.4 (KHTML, like Gecko) Mobile/14F89;GameHelper",
    "Mozilla/5.0 (iPhone; CPU iPhone OS 10_0 like Mac OS X) AppleWebKit/602.1.38 (KHTML, like Gecko) Version/10.0 Mobile/14A300 Safari/602.1",
    "Mozilla/5.0 (iPad; CPU OS 10_0 like Mac OS X) AppleWebKit/602.1.38 (KHTML, like Gecko) Version/10.0 Mobile/14A300 Safari/602.1",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10.12; rv:46.0) Gecko/20100101 Firefox/46.0",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_12_5) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/59.0.3071.115 Safari/537.36",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_12_5) AppleWebKit/603.2.4 (KHTML, like Gecko) Version/10.1.1 Safari/603.2.4",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:46.0) Gecko/20100101 Firefox/46.0",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/51.0.2704.103 Safari/537.36",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/42.0.2311.135 Safari/537.36 Edge/13.1058",
];

pub struct MusicApi {
    client: HttpClient,
    csrf: RefCell<String>,
}

#[allow(unused)]
enum CryptoApi {
    Weapi,
    LinuxApi,
    Eapi,
}

impl Default for MusicApi {
    fn default() -> Self {
        Self::new()
    }
}

impl MusicApi {
    #[allow(unused)]
    pub fn new() -> Self {
        let client = HttpClient::builder()
            .timeout(Duration::from_secs(20))
            .cookies()
            .build()
            .expect("???????????????????????????!");
        Self {
            client,
            csrf: RefCell::new(String::new()),
        }
    }

    #[allow(unused)]
    pub fn from_cookie_jar(cookie_jar: CookieJar) -> Self {
        let client = HttpClient::builder()
            .timeout(Duration::from_secs(20))
            .cookies()
            .cookie_jar(cookie_jar)
            .build()
            .expect("???????????????????????????!");
        Self {
            client,
            csrf: RefCell::new(String::new()),
        }
    }

    #[allow(unused)]
    pub fn cookie_jar(&self) -> Option<&CookieJar> {
        self.client.cookie_jar()
    }

    /// ??????????????????
    /// proxy: ?????????????????????????????????
    ///   - http: Proxy. Default when no scheme is specified.
    ///   - https: HTTPS Proxy. (Added in 7.52.0 for OpenSSL, GnuTLS and NSS)
    ///   - socks4: SOCKS4 Proxy.
    ///   - socks4a: SOCKS4a Proxy. Proxy resolves URL hostname.
    ///   - socks5: SOCKS5 Proxy.
    ///   - socks5h: SOCKS5 Proxy. Proxy resolves URL hostname.
    pub fn set_proxy(&mut self, proxy: &str) -> Result<()> {
        if let Some(cookie_jar) = self.client.cookie_jar() {
            let client = HttpClient::builder()
                .timeout(Duration::from_secs(20))
                .proxy(Some(proxy.parse()?))
                .cookie_jar(cookie_jar.to_owned())
                .cookies()
                .build()
                .expect("???????????????????????????!");
            self.client = client;
        } else {
            let client = HttpClient::builder()
                .timeout(Duration::from_secs(20))
                .proxy(Some(proxy.parse()?))
                .cookies()
                .build()
                .expect("???????????????????????????!");
            self.client = client;
        }
        Ok(())
    }

    /// ????????????
    /// method: ????????????
    /// path: ????????????
    /// params: ????????????
    /// cryptoapi: ??????????????????
    /// ua: ???????????? USER_AGENT_LIST
    /// append_csrf: ???????????????????????? csrf
    async fn request(
        &self,
        method: Method,
        path: &str,
        params: HashMap<&str, &str>,
        cryptoapi: CryptoApi,
        ua: &str,
        append_csrf: bool,
    ) -> Result<String> {
        let mut csrf = self.csrf.borrow().to_owned();
        if csrf.is_empty() {
            if let Some(cookies) = self.cookie_jar() {
                let uri = BASE_URL.parse().unwrap();
                if let Some(cookie) = cookies.get_by_name(&uri, "__csrf") {
                    let __csrf = cookie.value().to_string();
                    self.csrf.replace(__csrf.to_owned());
                    csrf = __csrf;
                }
            }
        }
        let mut url = format!("{}{}?csrf_token={}", BASE_URL, path, csrf);
        if !append_csrf {
            url = format!("{}{}", BASE_URL, path);
        }
        match method {
            Method::Post => {
                let user_agent = match cryptoapi {
                    CryptoApi::LinuxApi => LINUX_USER_AGNET.to_string(),
                    CryptoApi::Weapi => choose_user_agent(ua).to_string(),
                    CryptoApi::Eapi => choose_user_agent(ua).to_string(),
                };
                let body = match cryptoapi {
                    CryptoApi::LinuxApi => {
                        let data = format!(
                            r#"{{"method":"linuxapi","url":"{}","params":{}}}"#,
                            url.replace("weapi", "api"),
                            QueryParams::from_map(params).json()
                        );
                        Crypto::linuxapi(&data)
                    }
                    CryptoApi::Weapi => {
                        let mut params = params;
                        params.insert("csrf_token", &csrf);
                        Crypto::weapi(&QueryParams::from_map(params).json())
                    }
                    CryptoApi::Eapi => {
                        let mut params = params;
                        params.insert("csrf_token", &csrf);
                        url = path.to_string();
                        Crypto::eapi(
                            "/api/song/enhance/player/url",
                            &QueryParams::from_map(params).json(),
                        )
                    }
                };

                let request = Request::post(&url)
                    .header("Cookie", "os=pc; appver=2.7.1.198277")
                    .header("Accept", "*/*")
                    .header("Accept-Encoding", "gzip,deflate,br")
                    .header("Accept-Language", "en-US,en;q=0.5")
                    .header("Connection", "keep-alive")
                    .header("Content-Type", "application/x-www-form-urlencoded")
                    .header("Host", "music.163.com")
                    .header("Referer", "https://music.163.com")
                    .header("User-Agent", user_agent)
                    .body(body)
                    .unwrap();
                let mut response = self
                    .client
                    .send_async(request)
                    .await
                    .map_err(|_| anyhow!("none"))?;
                response.text().await.map_err(|_| anyhow!("none"))
            }
            Method::Get => self
                .client
                .get_async(&url)
                .await
                .map_err(|_| anyhow!("none"))?
                .text()
                .await
                .map_err(|_| anyhow!("none")),
        }
    }

    /// ??????
    /// username: ?????????(???????????????)
    /// password: ??????
    #[allow(unused)]
    pub async fn login(&self, username: String, password: String) -> Result<LoginInfo> {
        let mut params = HashMap::new();
        let path;
        if username.len().eq(&11) && username.parse::<u64>().is_ok() {
            path = "/weapi/login/cellphone";
            params.insert("phone", &username[..]);
            params.insert("password", &password[..]);
            params.insert("rememberLogin", "true");
        } else {
            let client_token =
                "1_jVUMqWEPke0/1/Vu56xCmJpo5vP1grjn_SOVVDzOc78w8OKLVZ2JH7IfkjSXqgfmh";
            path = "/weapi/login";
            params.insert("username", &username[..]);
            params.insert("password", &password[..]);
            params.insert("rememberLogin", "true");
            params.insert("clientToken", client_token);
        }
        let result = self
            .request(Method::Post, path, params, CryptoApi::Weapi, "", true)
            .await?;
        to_login_info(result)
    }

    /// ???????????????
    /// ctcode: ???????????????????????????????????????
    /// phone: ????????????
    /// captcha: ?????????
    #[allow(unused)]
    pub async fn login_cellphone(
        &self,
        ctcode: String,
        phone: String,
        captcha: String,
    ) -> Result<LoginInfo> {
        let path = "/weapi/login/cellphone";
        let mut params = HashMap::new();
        params.insert("phone", &phone[..]);
        params.insert("countrycode", &ctcode[..]);
        params.insert("captcha", &captcha[..]);
        params.insert("rememberLogin", "true");
        let result = self
            .request(Method::Post, path, params, CryptoApi::Weapi, "", true)
            .await?;
        to_login_info(result)
    }

    /// ???????????????
    /// ctcode: ???????????????????????????????????????
    /// phone: ????????????
    #[allow(unused)]
    pub async fn captcha(&self, ctcode: String, phone: String) -> Result<()> {
        let path = "/weapi/sms/captcha/sent";
        let mut params = HashMap::new();
        params.insert("cellphone", &phone[..]);
        params.insert("ctcode", &ctcode[..]);
        let result = self
            .request(Method::Post, path, params, CryptoApi::Weapi, "", true)
            .await?;
        to_captcha(result)
    }

    /// ???????????????????????????
    /// ??????(qr_url, unikey)
    #[allow(unused)]
    pub async fn login_qr_create(&self) -> Result<(String, String)> {
        let path = "/weapi/login/qrcode/unikey";
        let mut params = HashMap::new();
        params.insert("type", "1");
        let result = self
            .request(Method::Post, path, params, CryptoApi::Weapi, "", true)
            .await?;
        let unikey = to_unikey(result)?;
        Ok((
            format!("https://music.163.com/login?codekey={}", &unikey),
            unikey,
        ))
    }

    /// ?????????????????????
    /// key: ??? login_qr_create ????????? unikey
    #[allow(unused)]
    pub async fn login_qr_check(&self, key: String) -> Result<Msg> {
        let path = "/weapi/login/qrcode/client/login";
        let mut params = HashMap::new();
        params.insert("type", "1");
        params.insert("key", &key);
        let result = self
            .request(Method::Post, path, params, CryptoApi::Weapi, "", true)
            .await?;
        to_message(result)
    }

    /// ????????????
    #[allow(unused)]
    pub async fn login_status(&self) -> Result<LoginInfo> {
        let result = self
            .request(Method::Get, "", HashMap::new(), CryptoApi::Weapi, "", true)
            .await?;
        let re = regex::Regex::new(
            r#"userId:(?P<id>\d+),nickname:"(?P<nickname>\w+)",avatarUrl.+?(?P<avatar_url>http.+?jpg)""#,
        )?;
        let cap = re.captures(&result).ok_or_else(|| anyhow!("none"))?;
        let uid = cap
            .name("id")
            .ok_or_else(|| anyhow!("none"))?
            .as_str()
            .parse::<u64>()?;
        let nickname = cap
            .name("nickname")
            .ok_or_else(|| anyhow!("none"))?
            .as_str()
            .to_owned();
        let avatar_url = cap
            .name("avatar_url")
            .ok_or_else(|| anyhow!("none"))?
            .as_str()
            .to_owned();
        Ok(LoginInfo {
            code: 200,
            uid,
            nickname,
            avatar_url,
            msg: "?????????.".to_owned(),
        })
    }

    /// ??????
    #[allow(unused)]
    pub async fn logout(&self) {
        let path = "https://music.163.com/weapi/logout";
        self.request(
            Method::Post,
            path,
            HashMap::new(),
            CryptoApi::Weapi,
            "pc",
            true,
        )
        .await;
    }

    /// ????????????
    #[allow(unused)]
    pub async fn daily_task(&self) -> Result<Msg> {
        let path = "/weapi/point/dailyTask";
        let mut params = HashMap::new();
        params.insert("type", "0");
        let result = self
            .request(Method::Post, path, params, CryptoApi::Weapi, "", true)
            .await?;
        to_msg(result)
    }

    /// ????????????
    /// uid: ??????id
    /// offset: ???????????????
    /// limit: ????????????
    #[allow(unused)]
    pub async fn user_song_list(&self, uid: u64, offset: u16, limit: u16) -> Result<Vec<SongList>> {
        let path = "/weapi/user/playlist";
        let mut params = HashMap::new();
        let uid = uid.to_string();
        let offset = offset.to_string();
        let limit = limit.to_string();
        params.insert("uid", uid.as_str());
        params.insert("offset", offset.as_str());
        params.insert("limit", limit.as_str());
        let result = self
            .request(Method::Post, path, params, CryptoApi::Weapi, "", true)
            .await?;
        to_song_list(result, Parse::Usl)
    }

    /// ????????????????????????
    /// offset: ???????????????
    /// limit: ????????????
    #[allow(unused)]
    pub async fn album_sublist(&self, offset: u16, limit: u16) -> Result<Vec<SongList>> {
        let path = "/weapi/album/sublist";
        let mut params = HashMap::new();
        let offset = offset.to_string();
        let limit = limit.to_string();
        let total = true.to_string();
        params.insert("total", total.as_str());
        params.insert("offset", offset.as_str());
        params.insert("limit", limit.as_str());
        let result = self
            .request(Method::Post, path, params, CryptoApi::Weapi, "", true)
            .await?;
        to_song_list(result, Parse::LikeAlbum)
    }

    /// ????????????
    #[allow(unused)]
    pub async fn user_cloud_disk(&self) -> Result<Vec<SongInfo>> {
        let path = "/weapi/v1/cloud/get";
        let mut params = HashMap::new();
        params.insert("offset", "0");
        params.insert("limit", "10000");
        let result = self
            .request(Method::Post, path, params, CryptoApi::Weapi, "", true)
            .await?;
        to_song_info(result, Parse::Ucd)
    }

    /// ????????????
    /// songlist_id: ?????? id
    #[allow(unused)]
    pub async fn song_list_detail(&self, songlist_id: u64) -> Result<Vec<SongInfo>> {
        let csrf_token = self.csrf.borrow().to_owned();
        let path = "/weapi/v6/playlist/detail";
        let mut params = HashMap::new();
        let songlist_id = songlist_id.to_string();
        params.insert("id", songlist_id.as_str());
        params.insert("offset", "0");
        params.insert("total", "true");
        params.insert("limit", "1000");
        params.insert("n", "1000");
        params.insert("csrf_token", &csrf_token);
        let result = self
            .request(Method::Post, path, params, CryptoApi::Weapi, "", true)
            .await?;
        to_song_info(result, Parse::Usl)
    }

    /// ????????????
    /// ids: ?????? id ??????
    #[allow(unused)]
    pub async fn songs_detail(&self, ids: &[u64]) -> Result<Vec<SongInfo>> {
        let path = "/weapi/v3/song/detail";
        let mut params = HashMap::new();
        let c = ids
            .iter()
            .map(|i| format!("{{\\\"id\\\":\\\"{}\\\"}}", i))
            .collect::<Vec<String>>()
            .join(",");
        let c = format!("[{}]", c);
        params.insert("c", &c[..]);
        let result = self
            .request(Method::Post, path, params, CryptoApi::Weapi, "", true)
            .await?;
        to_song_info(result, Parse::Usl)
    }

    /// ?????? URL
    /// ids: ????????????
    /// br: ????????????
    ///     l: 128000
    ///     m: 192000
    ///     h: 320000
    ///    sq: 999000
    ///    hr: 1900000
    #[allow(unused)]
    pub async fn songs_url(&self, ids: &[u64], br: &str) -> Result<Vec<SongUrl>> {
        // ?????? WEBAPI ????????????
        // let csrf_token = self.csrf.borrow().to_owned();
        // let path = "/weapi/song/enhance/player/url/v1";
        // let mut params = HashMap::new();
        // let ids = serde_json::to_string(ids)?;
        // params.insert("ids", ids.as_str());
        // params.insert("level", "standard");
        // params.insert("encodeType", "aac");
        // params.insert("csrf_token", &csrf_token);
        // let result = self
        //     .request(Method::Post, path, params, CryptoApi::Weapi, "")
        //     .await?;

        // ?????? Eapi ????????????
        let path = "https://interface3.music.163.com/eapi/song/enhance/player/url";
        let mut params = HashMap::new();
        let ids = serde_json::to_string(ids)?;
        params.insert("ids", ids.as_str());
        params.insert("br", br);
        let result = self
            .request(Method::Post, path, params, CryptoApi::Eapi, "", true)
            .await?;
        to_song_url(result)
    }

    /// ??????????????????
    #[allow(unused)]
    pub async fn recommend_resource(&self) -> Result<Vec<SongList>> {
        let path = "/weapi/v1/discovery/recommend/resource";
        let result = self
            .request(
                Method::Post,
                path,
                HashMap::new(),
                CryptoApi::Weapi,
                "",
                true,
            )
            .await?;
        to_song_list(result, Parse::Rmd)
    }

    /// ??????????????????
    #[allow(unused)]
    pub async fn recommend_songs(&self) -> Result<Vec<SongInfo>> {
        let path = "/weapi/v2/discovery/recommend/songs";
        let mut params = HashMap::new();
        params.insert("total", "ture");
        let result = self
            .request(Method::Post, path, params, CryptoApi::Weapi, "", true)
            .await?;
        to_song_info(result, Parse::Rmds)
    }

    /// ??????FM
    #[allow(unused)]
    pub async fn personal_fm(&self) -> Result<Vec<SongInfo>> {
        let path = "/weapi/v1/radio/get";
        let result = self
            .request(
                Method::Post,
                path,
                HashMap::new(),
                CryptoApi::Weapi,
                "",
                true,
            )
            .await?;
        to_song_info(result, Parse::Rmd)
    }

    /// ??????/????????????
    /// songid: ??????id
    /// like: true ?????????false ??????
    #[allow(unused)]
    pub async fn like(&self, like: bool, songid: u64) -> bool {
        let path = "/weapi/radio/like";
        let mut params = HashMap::new();
        let songid = songid.to_string();
        let like = like.to_string();
        params.insert("alg", "itembased");
        params.insert("trackId", songid.as_str());
        params.insert("like", like.as_str());
        params.insert("time", "25");
        if let Ok(result) = self
            .request(Method::Post, path, params, CryptoApi::Weapi, "", true)
            .await
        {
            return to_msg(result)
                .unwrap_or(Msg {
                    code: 0,
                    msg: "".to_owned(),
                })
                .code
                .eq(&200);
        }
        false
    }

    /// FM ?????????
    /// songid: ??????id
    #[allow(unused)]
    pub async fn fm_trash(&self, songid: u64) -> bool {
        let path = "/weapi/radio/trash/add";
        let mut params = HashMap::new();
        let songid = songid.to_string();
        params.insert("alg", "RT");
        params.insert("songId", songid.as_str());
        params.insert("time", "25");
        if let Ok(result) = self
            .request(Method::Post, path, params, CryptoApi::Weapi, "", true)
            .await
        {
            return to_msg(result)
                .unwrap_or(Msg {
                    code: 0,
                    msg: "".to_owned(),
                })
                .code
                .eq(&200);
        }
        false
    }

    /// ??????
    /// keywords: ?????????
    /// types: 1: ??????, 10: ??????, 100: ??????, 1000: ??????, 1002: ??????, 1004: MV, 1006: ??????, 1009: ??????, 1014: ??????
    /// offset: ?????????
    /// limit: ??????
    #[allow(unused)]
    pub async fn search(
        &self,
        keywords: String,
        types: u32,
        offset: u16,
        limit: u16,
    ) -> Result<String> {
        let path = "/weapi/search/get";
        let mut params = HashMap::new();
        let _types = types.to_string();
        let offset = offset.to_string();
        let limit = limit.to_string();
        params.insert("s", &keywords[..]);
        params.insert("type", &_types[..]);
        params.insert("offset", &offset[..]);
        params.insert("limit", &limit[..]);
        self.request(Method::Post, path, params, CryptoApi::Weapi, "", true)
            .await
    }

    /// ????????????
    /// keywords: ?????????
    /// offset: ?????????
    /// limit: ??????
    #[allow(unused)]
    pub async fn search_song(
        &self,
        keywords: String,
        offset: u16,
        limit: u16,
    ) -> Result<Vec<SongInfo>> {
        let result = self.search(keywords, 1, offset, limit).await?;
        to_song_info(result, Parse::Search)
    }

    /// ????????????
    /// keywords: ?????????
    /// offset: ?????????
    /// limit: ??????
    #[allow(unused)]
    pub async fn search_singer(
        &self,
        keywords: String,
        offset: u16,
        limit: u16,
    ) -> Result<Vec<SingerInfo>> {
        let result = self.search(keywords, 100, offset, limit).await?;
        to_singer_info(result)
    }

    /// ????????????
    /// keywords: ?????????
    /// offset: ?????????
    /// limit: ??????
    #[allow(unused)]
    pub async fn search_album(
        &self,
        keywords: String,
        offset: u16,
        limit: u16,
    ) -> Result<Vec<SongList>> {
        let result = self.search(keywords, 10, offset, limit).await?;
        to_song_list(result, Parse::SearchAlbum)
    }

    /// ????????????
    /// keywords: ?????????
    /// offset: ?????????
    /// limit: ??????
    #[allow(unused)]
    pub async fn search_songlist(
        &self,
        keywords: String,
        offset: u16,
        limit: u16,
    ) -> Result<Vec<SongList>> {
        let result = self.search(keywords, 1000, offset, limit).await?;
        to_song_list(result, Parse::Search)
    }

    /// ????????????
    /// keywords: ?????????
    /// offset: ?????????
    /// limit: ??????
    #[allow(unused)]
    pub async fn search_lyrics(
        &self,
        keywords: String,
        offset: u16,
        limit: u16,
    ) -> Result<Vec<SongInfo>> {
        let result = self.search(keywords, 1006, offset, limit).await?;
        to_song_info(result, Parse::Search)
    }

    /// ??????????????????
    /// id: ?????? ID
    #[allow(unused)]
    pub async fn singer_songs(&self, id: u64) -> Result<Vec<SongInfo>> {
        let path = format!("/weapi/v1/artist/{}", id);
        let mut params = HashMap::new();
        let result = self
            .request(Method::Post, &path, params, CryptoApi::Weapi, "", false)
            .await?;
        to_song_info(result, Parse::Singer)
    }

    /// ????????????
    /// offset: ?????????
    /// limit: ??????
    /// area: ALL:??????,ZH:??????,EA:??????,KR:??????,JP:??????
    #[allow(unused)]
    pub async fn new_albums(&self, area: &str, offset: u16, limit: u16) -> Result<Vec<SongList>> {
        let path = "/weapi/album/new";
        let mut params = HashMap::new();
        let offset = offset.to_string();
        let limit = limit.to_string();
        params.insert("area", area);
        params.insert("offset", &offset[..]);
        params.insert("limit", &limit[..]);
        params.insert("total", "true");
        let result = self
            .request(Method::Post, path, params, CryptoApi::Weapi, "", true)
            .await?;
        to_song_list(result, Parse::Album)
    }

    /// ??????
    /// album_id: ?????? id
    #[allow(unused)]
    pub async fn album(&self, album_id: u64) -> Result<Vec<SongInfo>> {
        let path = format!("/weapi/v1/album/{}", album_id);
        let result = self
            .request(
                Method::Post,
                &path,
                HashMap::new(),
                CryptoApi::Weapi,
                "",
                true,
            )
            .await?;
        to_song_info(result, Parse::Album)
    }

    /// ??????????????????
    /// offset: ?????????
    /// limit: ??????
    /// order: ????????????:
    //	      "hot": ?????????
    ///        "new": ??????
    /// cat: ??????,??????,??????,??????,??????,??????,?????????,??????,??????,??????,??????,??????,??????,?????????,??????,??????,R&B/Soul,??????,??????,??????,??????,??????,??????,??????,????????????,??????,??????/??????,New Age,??????,??????,Bossa Nova,??????,??????,??????,??????,??????,?????????,??????,??????,??????,??????,??????,??????,??????,??????,??????,??????,??????,??????,??????,??????,??????,??????,??????,??????,??????,????????????,ACG,??????,??????,??????,70???,80???,90???,????????????,KTV,??????,??????,??????,??????,??????,??????,00???
    #[allow(unused)]
    pub async fn top_song_list(
        &self,
        cat: &str,
        order: &str,
        offset: u16,
        limit: u16,
    ) -> Result<Vec<SongList>> {
        let path = "/weapi/playlist/list";
        let mut params = HashMap::new();
        let offset = offset.to_string();
        let limit = limit.to_string();
        params.insert("cat", cat);
        params.insert("order", order);
        params.insert("total", "true");
        params.insert("offset", &offset[..]);
        params.insert("limit", &limit[..]);
        let result = self
            .request(Method::Post, path, params, CryptoApi::Weapi, "", true)
            .await?;
        to_song_list(result, Parse::Top)
    }

    /// ????????????
    /// lasttime: ????????????,????????????????????????????????? updateTime ?????????????????????
    /// limit: ??????
    /// cat: ??????,??????,??????,??????,??????,??????,?????????,??????,ACG,????????????,??????,??????,??????,??????,??????,?????????,??????,??????,??????,??????,??????
    #[allow(unused)]
    pub async fn top_song_list_highquality(
        &self,
        cat: &str,
        lasttime: u8,
        limit: u8,
    ) -> Result<Vec<SongList>> {
        let path = "/api/playlist/highquality/list";
        let mut params = HashMap::new();
        let lasttime = lasttime.to_string();
        let limit = limit.to_string();
        params.insert("cat", cat);
        params.insert("total", "true");
        params.insert("lasttime", &lasttime[..]);
        params.insert("limit", &limit[..]);
        let result = self
            .request(Method::Post, path, params, CryptoApi::Weapi, "", true)
            .await?;
        to_song_list(result, Parse::Top)
    }

    /// ???????????????
    #[allow(unused)]
    pub async fn toplist(&self) -> Result<Vec<TopList>> {
        let path = "/api/toplist";
        let mut params = HashMap::new();
        let res = self
            .request(Method::Post, path, params, CryptoApi::Weapi, "", true)
            .await?;
        to_toplist(res)
    }

    /// ????????????/?????????
    /// list_id:
    /// ??????????????????: 19723756
    /// ??????????????????: 3779629
    /// ?????????????????????: 2884035
    /// ??????????????????: 3778678
    /// ????????????????????????: 71384707
    /// ?????????ACG?????????: 71385702
    /// ??????????????????: 745956260
    /// ??????????????????: 10520166
    /// ??????????????????: 991319590']
    /// ???????????????: 2250011882
    /// UK???????????????: 180106
    /// ??????Billboard??????: 60198
    /// KTV??????: 21845217
    /// iTunes???: 11641012
    /// Hit FM Top???: 120001
    /// ??????Oricon??????: 60131
    /// ??????Hito?????????: 112463
    /// ?????????????????????????????????: 10169002
    /// ???????????????: 4395559
    #[allow(unused)]
    pub async fn top_songs(&self, list_id: u64) -> Result<Vec<SongInfo>> {
        self.song_list_detail(list_id).await
    }

    /// ????????????
    /// music_id: ??????id
    #[allow(unused)]
    pub async fn song_lyric(&self, music_id: u64) -> Result<Vec<String>> {
        let csrf_token = self.csrf.borrow().to_owned();
        let path = "/weapi/song/lyric";
        let mut params = HashMap::new();
        let id = music_id.to_string();
        params.insert("id", &id[..]);
        params.insert("lv", "-1");
        params.insert("tv", "-1");
        params.insert("csrf_token", &csrf_token);
        let result = self
            .request(Method::Post, path, params, CryptoApi::Weapi, "", true)
            .await?;
        to_lyric(result)
    }

    /// ??????/??????????????????
    /// like: true ?????????false ??????
    /// id: ?????? id
    #[allow(unused)]
    pub async fn song_list_like(&self, like: bool, id: u64) -> bool {
        let path = if like {
            "/weapi/playlist/subscribe"
        } else {
            "/weapi/playlist/unsubscribe"
        };
        let mut params = HashMap::new();
        let id = id.to_string();
        params.insert("id", &id[..]);
        if let Ok(result) = self
            .request(Method::Post, path, params, CryptoApi::Weapi, "", true)
            .await
        {
            return to_msg(result)
                .unwrap_or(Msg {
                    code: 0,
                    msg: "".to_owned(),
                })
                .code
                .eq(&200);
        }
        false
    }

    /// ??????/??????????????????
    /// like: true ?????????false ??????
    /// id: ?????? id
    #[allow(unused)]
    pub async fn album_like(&self, like: bool, id: u64) -> bool {
        let path = if like {
            "/api/album/sub"
        } else {
            "/api/album/unsub"
        };
        let path = format!("{}?id={}", path, id);
        let mut params = HashMap::new();
        let id = id.to_string();
        params.insert("id", id.as_str());
        if let Ok(result) = self
            .request(Method::Post, &path, params, CryptoApi::Weapi, "", false)
            .await
        {
            return to_msg(result)
                .unwrap_or(Msg {
                    code: 0,
                    msg: "".to_owned(),
                })
                .code
                .eq(&200);
        }
        false
    }

    /// ?????? APP ????????????
    #[allow(unused)]
    pub async fn homepage(&self, client_type: ClientType) -> Result<String> {
        let path = "/api/homepage/block/page";
        let mut params = HashMap::new();
        params.insert("refresh", "false");
        params.insert("cursor", "null");
        self.request(Method::Post, path, params, CryptoApi::Weapi, "", true)
            .await
    }

    /// ?????????????????????
    #[allow(unused)]
    pub async fn banners(&self) -> Result<Vec<BannersInfo>> {
        to_banners_info(self.homepage(ClientType::Iphone).await?)
    }

    /// ?????????????????????
    /// url: ??????
    /// path: ??????????????????(???????????????)
    /// width: ??????
    /// high: ??????
    #[allow(unused)]
    pub async fn download_img<I>(&self, url: I, path: PathBuf, width: u16, high: u16) -> Result<()>
    where
        I: Into<String>,
    {
        if !path.exists() {
            let url = url.into();
            let image_url = format!("{}?param={}y{}", url, width, high);

            let mut response = self.client.get_async(image_url).await?;
            if response.status().is_success() {
                let mut buf = vec![];
                response.copy_to(&mut buf).await?;
                std::fs::write(&path, buf)?;
            }
        }
        Ok(())
    }

    /// ?????????????????????
    /// url: ??????
    /// path: ??????????????????(???????????????)
    #[allow(unused)]
    pub async fn download_song<I>(&self, url: I, path: PathBuf) -> Result<()>
    where
        I: Into<String>,
    {
        if !path.exists() {
            let mut response = self.client.get_async(url.into()).await?;
            if response.status().is_success() {
                let mut buf = vec![];
                response.copy_to(&mut buf).await?;
                std::fs::write(&path, buf)?;
            }
        }
        Ok(())
    }
}

fn choose_user_agent(ua: &str) -> &str {
    let index = if ua == "mobile" {
        rand::random::<usize>() % 7
    } else if ua == "pc" {
        rand::random::<usize>() % 5 + 8
    } else if !ua.is_empty() {
        return ua;
    } else {
        rand::random::<usize>() % USER_AGENT_LIST.len()
    };
    USER_AGENT_LIST[index]
}

#[cfg(test)]
mod tests {

    use super::*;

    #[async_std::test]
    async fn test() {
        let api = MusicApi::new();
        assert!(api.banners().await.is_ok());
    }
}
