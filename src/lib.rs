//
// mod.rs
// Copyright (C) 2019 gmg137 <gmg137@live.com>
// Distributed under terms of the GPLv3 license.
//
mod encrypt;
pub(crate) mod model;
use anyhow::{anyhow, Result};
use encrypt::{Crypto, XeapiPublicKeyState};
pub use isahc::cookies::{CookieBuilder, CookieJar};
use isahc::{prelude::*, *};
use lazy_static::lazy_static;
pub use model::*;
use regex::Regex;
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, OnceLock, RwLock},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use urlqstring::QueryParams;

lazy_static! {
    static ref _CSRF: Regex = Regex::new(r"_csrf=(?P<csrf>[^(;|$)]+)").unwrap();
    /// 易盾 v3 反作弊 token 的 JSONP 响应: null([200,<时间戳>,"<token>"])
    static ref _CHECK_TOKEN: Regex = Regex::new(r#"null\(\[(\d+),\d+,"([^"]+)"\]\)"#).unwrap();
}

static BASE_URL: &str = "https://music.163.com";

const TIMEOUT: u64 = 100;

const LINUX_USER_AGNET: &str =
    "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/60.0.3112.90 Safari/537.36";

const XEAPI_USER_AGENT: &str =
    "NeteaseMusic/9.5.61.260802021928(9005061);Dalvik/2.1.0 (Linux; U; Android 12; HBN-AL00 Build/cd737a2.0)";

/// 易盾 v3 反作弊 token 注册端点，每次请求实时获取，不缓存
static DUN_V3_URL: &str = "https://ac.dun.163yun.com/v3/b?pn=YD00000558929251";

/// 收藏歌单时随请求体下发的静态 checkToken
static CHECK_TOKEN: &str = "9ca17ae2e6ffcda170e2e6ee8af14fbabdb988f225b3868eb2c15a879b9a83d274a790ac8ff54a97b889d5d42ad0feaec3b92af58cff99c470a7eafd88f75e839a9ea7c14e909da883e83fb692a3abdb6b92adee9e";

/// eapi 请求携带反作弊 token 的头字段名
static ANTI_CHEAT_HEADER: &str = "X-antiCheatToken";

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

#[derive(Clone)]
pub struct MusicApi {
    client: HttpClient,
    csrf: OnceLock<String>,
    xeapi_public_key: OnceLock<XeapiPublicKeyState>,
    /// 外部注入的反作弊 token，被下一次 eapi 请求消费
    anti_cheat_override: Arc<RwLock<Option<String>>>,
}

#[allow(unused)]
enum CryptoApi {
    Weapi,
    LinuxApi,
    Eapi,
    Xeapi,
}

impl Default for MusicApi {
    fn default() -> Self {
        Self::new(0)
    }
}

impl MusicApi {
    #[allow(unused)]
    pub fn new(max_cons: usize) -> Self {
        let client = HttpClient::builder()
            .timeout(Duration::from_secs(TIMEOUT))
            .max_connections(max_cons)
            .cookies()
            .build()
            .expect("初始化网络请求失败!");
        Self {
            client,
            csrf: OnceLock::new(),
            xeapi_public_key: OnceLock::new(),
            anti_cheat_override: Arc::new(RwLock::new(None)),
        }
    }

    #[allow(unused)]
    pub fn from_cookie_jar(cookie_jar: CookieJar, max_cons: usize) -> Self {
        let client = HttpClient::builder()
            .timeout(Duration::from_secs(TIMEOUT))
            .max_connections(max_cons)
            .cookies()
            .cookie_jar(cookie_jar)
            .build()
            .expect("初始化网络请求失败!");
        Self {
            client,
            csrf: OnceLock::new(),
            xeapi_public_key: OnceLock::new(),
            anti_cheat_override: Arc::new(RwLock::new(None)),
        }
    }

    #[allow(unused)]
    pub fn cookie_jar(&self) -> Option<&CookieJar> {
        self.client.cookie_jar()
    }

    /// 设置使用代理
    /// proxy: 代理地址，支持以下协议
    ///   - http: Proxy. Default when no scheme is specified.
    ///   - https: HTTPS Proxy. (Added in 7.52.0 for OpenSSL, GnuTLS and NSS)
    ///   - socks4: SOCKS4 Proxy.
    ///   - socks4a: SOCKS4a Proxy. Proxy resolves URL hostname.
    ///   - socks5: SOCKS5 Proxy.
    ///   - socks5h: SOCKS5 Proxy. Proxy resolves URL hostname.
    pub fn set_proxy(&mut self, proxy: &str) -> Result<()> {
        if let Some(cookie_jar) = self.client.cookie_jar() {
            let client = HttpClient::builder()
                .timeout(Duration::from_secs(TIMEOUT))
                .proxy(Some(proxy.parse()?))
                .cookies()
                .cookie_jar(cookie_jar.to_owned())
                .build()
                .expect("初始化网络请求失败!");
            self.client = client;
        } else {
            let client = HttpClient::builder()
                .timeout(Duration::from_secs(TIMEOUT))
                .proxy(Some(proxy.parse()?))
                .cookies()
                .build()
                .expect("初始化网络请求失败!");
            self.client = client;
        }
        Ok(())
    }

    /// 读取 cookie jar 中指定 cookie 的值
    fn get_cookie(&self, name: &str) -> Option<String> {
        let cookies = self.cookie_jar()?;
        let uri = BASE_URL.parse().ok()?;
        let value = cookies.get_by_name(&uri, name)?.value().to_string();
        if value.is_empty() {
            None
        } else {
            Some(value)
        }
    }

    fn get_music_u(&self) -> Option<String> {
        self.get_cookie("MUSIC_U")
    }

    /// 注入一次性反作弊 token，仅对下一次 eapi 请求生效
    /// token: 易盾 token，可在浏览器侧获取后传入
    #[allow(unused)]
    pub fn set_anti_cheat_token(&self, token: &str) {
        if let Ok(mut slot) = self.anti_cheat_override.write() {
            *slot = Some(token.to_string());
        }
    }

    /// 获取易盾 v3 反作弊 token，优先使用外部注入值
    async fn fetch_anti_cheat_token(&self) -> Option<String> {
        let injected = self
            .anti_cheat_override
            .write()
            .ok()
            .and_then(|mut slot| slot.take());
        if let Some(token) = injected {
            return Some(token);
        }
        let body = self
            .client
            .get_async(DUN_V3_URL)
            .await
            .ok()?
            .text()
            .await
            .ok()?;
        parse_anti_cheat_token(&body)
    }

    /// eapi 请求的 Cookie 头，在基础 cookie 上补登录态与附加头
    fn eapi_cookie(&self, extra_headers: &[(&str, &str)]) -> String {
        let mut cookie = build_cookie(&CryptoApi::Eapi);
        for name in ["MUSIC_U", "MUSIC_A"] {
            if let Some(value) = self.get_cookie(name) {
                cookie.push_str(&format!("; {}={}", name, value));
            }
        }
        for (name, value) in extra_headers {
            cookie.push_str(&format!("; {}={}", name, value));
        }
        cookie
    }

    /// 获取 xeapi 公钥
    #[allow(unused)]
    pub async fn fetch_xeapi_public_key(&self) -> Result<()> {
        if self.xeapi_public_key.get().is_some() {
            return Ok(());
        }
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis()
            .to_string();
        let nonce: String = (0..16)
            .map(|_| (b'0' + rand::random::<u8>() % 10) as char)
            .collect();
        let signature = Crypto::xeapi_sign(&timestamp, &nonce);
        let device_id = format!("{:x}{:x}", rand::random::<u64>(), rand::random::<u64>());
        let body = QueryParams::from(vec![
            ("appVersion", "9.5.61"),
            ("currentKeyVersion", ""),
            ("deviceId", device_id.as_str()),
            ("nonce", nonce.as_str()),
            ("os", "android"),
            ("requestType", "active"),
            ("signature", signature.as_str()),
            ("t1", ""),
            ("t2", ""),
            ("timestamp", timestamp.as_str()),
            ("uid", ""),
        ])
        .stringify();
        let url = "https://interface.music.163.com/api/gorilla/anti/crawler/security/key/get";
        let request = Request::post(url)
            .header("User-Agent", XEAPI_USER_AGENT)
            .header(
                "Content-Type",
                "application/x-www-form-urlencoded;charset=utf-8",
            )
            .header("Host", "interface.music.163.com")
            .header("Cookie", build_cookie(&CryptoApi::Xeapi))
            .body(body)
            .unwrap();
        let mut response = self
            .client
            .send_async(request)
            .await
            .map_err(|_| anyhow!("none"))?;
        let text = response.text().await.map_err(|_| anyhow!("none"))?;
        let json: serde_json::Value =
            serde_json::from_str(&text).map_err(|_| anyhow!("json parse failed"))?;
        let data = json.get("data").ok_or_else(|| anyhow!("no data"))?;
        let encrypted_data = data
            .get("encryptedData")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("no encryptedData"))?;
        let public_key = Crypto::xeapi_decrypt_public_key(encrypted_data)?;
        let _ = self.xeapi_public_key.get_or_init(|| public_key);
        Ok(())
    }

    /// 发送请求
    /// method: 请求方法
    /// path: 请求路径
    /// params: 请求参数
    /// cryptoapi: 请求加密方式
    /// ua: 要使用的 USER_AGENT_LIST
    /// append_csrf: 是否在路径中添加 csrf
    async fn request(
        &self,
        method: Method,
        path: &str,
        params: HashMap<&str, &str>,
        cryptoapi: CryptoApi,
        ua: &str,
        append_csrf: bool,
    ) -> Result<String> {
        self.request_ext(method, path, params, cryptoapi, ua, append_csrf, &[])
            .await
    }

    /// 发送请求，可向 eapi 加密体的 header 追加字段
    /// extra_headers: 追加的字段名与值，同时写入 Cookie 头
    // 参数与 request 保持一致，额外多一个头字段，拆分结构体不值得
    #[allow(clippy::too_many_arguments)]
    async fn request_ext(
        &self,
        method: Method,
        path: &str,
        params: HashMap<&str, &str>,
        cryptoapi: CryptoApi,
        ua: &str,
        append_csrf: bool,
        extra_headers: &[(&str, &str)],
    ) -> Result<String> {
        let empty = String::new();
        let csrf = match self.csrf.get() {
            Some(str) => str.clone(),
            None => {
                if let Some(cookies) = self.cookie_jar() {
                    let uri = BASE_URL.parse().unwrap();
                    if let Some(cookie) = cookies.get_by_name(&uri, "__csrf") {
                        let str = cookie.value();
                        if !str.is_empty() {
                            self.csrf.get_or_init(|| str.to_string());
                        }
                    }
                }
                self.csrf.get().unwrap_or(&empty).clone()
            }
        };
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
                    CryptoApi::Xeapi => XEAPI_USER_AGENT.to_string(),
                };
                let body = match cryptoapi {
                    CryptoApi::LinuxApi => {
                        let data = serde_json::to_string(&serde_json::json!({
                            "method": "linuxapi",
                            "url": url.replace("weapi", "api"),
                            "params": params,
                        }))
                        .unwrap();
                        Crypto::linuxapi(&data)
                    }
                    CryptoApi::Weapi => {
                        let mut params = params;
                        params.insert("csrf_token", &csrf);
                        Crypto::weapi(&serde_json::to_string(&params).unwrap())
                    }
                    CryptoApi::Eapi => {
                        let mut params = params;
                        params.insert("csrf_token", &csrf);

                        let api_path = path.replacen("/api", "/eapi", 1);
                        url = format!("https://interface.music.163.com{}", api_path);

                        let mut data = serde_json::json!(params);

                        let now_ms = SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap()
                            .as_millis();
                        let buildver = now_ms.to_string();
                        let buildver = &buildver[..buildver.len().min(10)];
                        let request_id = format!("{}_{:04}", now_ms, rand::random::<u16>() % 1000);

                        let mut header = serde_json::json!({
                            "osver": "16.2",
                            "deviceId": "",
                            "os": "iPhone OS",
                            "appver": "9.0.90",
                            "versioncode": "140",
                            "mobilename": "",
                            "buildver": buildver,
                            "resolution": "1920x1080",
                            "__csrf": csrf,
                            "channel": "",
                            "requestId": request_id,
                        });

                        // eapi 通过加密体内的 header 携带登录态与反作弊 token
                        if let serde_json::Value::Object(ref mut map) = header {
                            for name in ["MUSIC_U", "MUSIC_A"] {
                                if let Some(value) = self.get_cookie(name) {
                                    map.insert(name.to_string(), value.into());
                                }
                            }
                            for (name, value) in extra_headers {
                                map.insert((*name).to_string(), (*value).into());
                            }
                        }

                        if let serde_json::Value::Object(ref mut map) = data {
                            map.insert("header".to_string(), header);
                        }

                        Crypto::eapi(path, &data.to_string())
                    }
                    CryptoApi::Xeapi => {
                        if self.xeapi_public_key.get().is_none() {
                            self.fetch_xeapi_public_key().await?;
                        }
                        let pk = self
                            .xeapi_public_key
                            .get()
                            .ok_or_else(|| anyhow!("xeapi public key unavailable"))?;
                        let rest = if path.starts_with("/api/") {
                            &path[5..]
                        } else {
                            path
                        };
                        url = format!("https://interface3.music.163.com/xeapi/{}", rest);
                        Crypto::xeapi(path, &params, pk)?
                    }
                };

                let host = match cryptoapi {
                    CryptoApi::Eapi => "interface.music.163.com",
                    CryptoApi::Xeapi => "interface3.music.163.com",
                    _ => "music.163.com",
                };
                let cookie = match cryptoapi {
                    CryptoApi::Eapi => self.eapi_cookie(extra_headers),
                    _ => build_cookie(&cryptoapi),
                };
                let request = Request::post(&url)
                    .header("Cookie", cookie)
                    .header("Accept", "*/*")
                    .header("Accept-Language", "en-US,en;q=0.5")
                    .header("Connection", "keep-alive")
                    .header("Content-Type", "application/x-www-form-urlencoded")
                    .header("Host", host)
                    .header("Referer", "https://music.163.com")
                    .header("User-Agent", user_agent);
                let request = if let CryptoApi::Xeapi = cryptoapi {
                    let now_ms = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_millis();
                    let now_str = now_ms.to_string();
                    let buildver = &now_str[..now_str.len().min(10)];
                    let device_id =
                        format!("{:x}{:x}", rand::random::<u64>(), rand::random::<u64>());
                    let mut rb = request
                        .header(
                            "Content-Type",
                            "application/x-www-form-urlencoded;charset=utf-8",
                        )
                        .header("X-Client-Enc-State", "ENCRYPTED")
                        .header("x-aeapi", "true")
                        .header("x-deviceid", &device_id)
                        .header("x-os", "android")
                        .header("x-osver", "16")
                        .header("x-appver", "9.5.61")
                        .header("x-sdeviceid", &device_id)
                        .header("x-buildver", buildver);
                    if let Some(music_u) = self.get_music_u() {
                        rb = rb.header("x-music-u", &music_u);
                    }
                    rb.body(body).unwrap()
                } else {
                    request.body(body).unwrap()
                };
                let mut response = self
                    .client
                    .send_async(request)
                    .await
                    .map_err(|_| anyhow!("none"))?;
                match cryptoapi {
                    CryptoApi::Xeapi => {
                        let bytes = response.bytes().await.map_err(|_| anyhow!("none"))?;
                        Crypto::xeapi_res_decrypt(&bytes)
                    }
                    _ => response.text().await.map_err(|_| anyhow!("none")),
                }
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

    /// 登录
    /// username: 用户名(邮箱或手机)
    /// password: 密码
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

    /// 手机号登录
    /// ctcode: 国家码，用于国外手机号登录
    /// phone: 手机号码
    /// captcha: 验证码
    #[allow(unused)]
    pub async fn login_cellphone(
        &self,
        ctcode: String,
        phone: String,
        captcha: String,
    ) -> Result<LoginInfo> {
        let path = "/weapi/w/login/cellphone";
        let mut params = HashMap::new();
        params.insert("phone", &phone[..]);
        params.insert("countrycode", &ctcode[..]);
        params.insert("type", "1");
        params.insert("https", "true");
        params.insert("remember", "true");
        params.insert("captcha", &captcha[..]);
        let result = self
            .request(Method::Post, path, params, CryptoApi::Weapi, "", true)
            .await?;
        to_login_info(result)
    }

    /// 获取验证码
    /// ctcode: 国家码，用于国外手机号登录
    /// phone: 手机号码
    #[allow(unused)]
    pub async fn captcha(&self, ctcode: String, phone: String) -> Result<()> {
        let path = "/weapi/sms/captcha/sent";
        let mut params = HashMap::new();
        params.insert("cellphone", &phone[..]);
        params.insert("ctcode", &ctcode[..]);
        params.insert("secrete", "music_middleuser_pclogin");
        let result = self
            .request(Method::Post, path, params, CryptoApi::Weapi, "", true)
            .await?;
        to_captcha(result)
    }

    /// 创建登陆二维码链接
    /// 返回(qr_url, unikey)
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

    /// 检查登陆二维码
    /// key: 由 login_qr_create 生成的 unikey
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

    /// 登录状态
    #[allow(unused)]
    pub async fn login_status(&self) -> Result<LoginInfo> {
        let path = "/api/nuser/account/get";
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
        to_login_info(result)
    }

    /// 退出
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

    /// 每日签到
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

    /// 用户喜欢音乐id列表
    /// uid: 用户id
    #[allow(unused)]
    pub async fn user_song_id_list(&self, uid: u64) -> Result<Vec<u64>> {
        let path = "/weapi/song/like/get";
        let mut params = HashMap::new();
        let uid = uid.to_string();
        params.insert("uid", uid.as_str());
        let result = self
            .request(Method::Post, path, params, CryptoApi::Weapi, "", true)
            .await?;
        to_song_id_list(result)
    }

    /// 用户歌单
    /// uid: 用户id
    /// offset: 列表起点号
    /// limit: 列表长度
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

    /// 用户收藏专辑列表
    /// offset: 列表起点号
    /// limit: 列表长度
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

    /// 用户云盘
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

    /// 歌单详情
    /// songlist_id: 歌单 id
    #[allow(unused)]
    pub async fn song_list_detail(&self, songlist_id: u64) -> Result<PlayListDetail> {
        let path = "/api/v6/playlist/detail";
        let mut params = HashMap::new();
        let songlist_id = songlist_id.to_string();
        params.insert("id", songlist_id.as_str());
        params.insert("n", "10000");
        params.insert("s", "8");
        let result = self
            .request(Method::Post, path, params, CryptoApi::Eapi, "", true)
            .await?;
        let json: serde_json::Value = serde_json::from_str(&result)?;
        complete_playlist(&json, |ids| async move { self.songs_detail(&ids).await }).await
    }

    /// 歌曲详情
    /// ids: 歌曲 id 列表
    #[allow(unused)]
    pub async fn songs_detail(&self, ids: &[u64]) -> Result<Vec<SongInfo>> {
        let path = "/weapi/v3/song/detail";
        let mut params = HashMap::new();
        let c = serde_json::to_string(
            &ids.iter()
                .map(|id| serde_json::json!({ "id": id }))
                .collect::<Vec<_>>(),
        )?;
        params.insert("c", &c[..]);
        let result = self
            .request(Method::Post, path, params, CryptoApi::Weapi, "", true)
            .await?;
        to_song_info(result, Parse::Usl)
    }

    /// 歌曲 URL
    /// ids: 歌曲列表
    /// br: 歌曲码率
    ///     l: 128000
    ///     m: 192000
    ///     h: 320000
    ///    sq: 999000
    ///    hr: 1900000
    #[allow(unused)]
    pub async fn songs_url(&self, ids: &[u64], br: &str) -> Result<Vec<SongUrl>> {
        let path = "/api/song/enhance/player/url";
        let mut params = HashMap::new();
        let ids = serde_json::to_string(ids)?;
        params.insert("ids", ids.as_str());
        params.insert("br", br);
        let result = self
            .request(Method::Post, path, params, CryptoApi::Eapi, "", true)
            .await?;
        to_song_url(result)
    }

    /// 歌曲 URL v1 (基于音质等级)
    /// ids: 歌曲列表
    /// level: 音质等级 (Standard, Higher, Extreme, Lossless, HiRes, Surround, AudioVivid, Master)
    #[allow(unused)]
    pub async fn songs_url_v1(&self, ids: &[u64], level: SongQuality) -> Result<Vec<SongUrl>> {
        let path = "/api/song/enhance/player/url/v1";
        let mut params = HashMap::new();
        let ids = serde_json::to_string(ids)?;
        let level_value = level.as_level().to_string();
        let encode_type = if matches!(
            level,
            SongQuality::Standard | SongQuality::Higher | SongQuality::Extreme
        ) {
            "aac".to_string()
        } else {
            "flac".to_string()
        };
        params.insert("ids", &ids[..]);
        params.insert("level", &level_value[..]);
        params.insert("encodeType", &encode_type[..]);
        if matches!(level, SongQuality::AudioVivid) {
            params.insert("immerseType", "c51");
        }
        let result = self
            .request(Method::Post, path, params, CryptoApi::Eapi, "", true)
            .await?;
        to_song_url(result)
    }

    /// 每日推荐歌单
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

    /// 每日推荐歌曲
    #[allow(unused)]
    pub async fn recommend_songs(&self) -> Result<Vec<SongInfo>> {
        let path = "/api/v3/discovery/recommend/songs";
        let mut params = HashMap::new();
        params.insert("afresh", "false");
        let result = self
            .request(Method::Post, path, params, CryptoApi::Weapi, "", true)
            .await?;
        to_song_info(result, Parse::Rmds)
    }

    /// 私人FM
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

    /// 收藏/取消收藏
    /// songid: 歌曲id
    /// like: true 收藏，false 取消
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

    /// 收藏/取消收藏 (xeapi)
    /// songid: 歌曲id
    /// like: true 收藏，false 取消
    #[allow(unused)]
    pub async fn like_v1(&self, like: bool, songid: u64) -> bool {
        let path = "/api/v1/radio/like";
        let mut params = HashMap::new();
        let songid = songid.to_string();
        let like = like.to_string();
        params.insert("alg", "itembased");
        params.insert("trackId", songid.as_str());
        params.insert("like", like.as_str());
        params.insert("time", "3");
        if let Ok(result) = self
            .request(Method::Post, path, params, CryptoApi::Xeapi, "", false)
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

    /// FM 不喜欢
    /// songid: 歌曲id
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

    /// 搜索
    /// keywords: 关键词
    /// types: 1: 单曲, 10: 专辑, 100: 歌手, 1000: 歌单, 1002: 用户, 1004: MV, 1006: 歌词, 1009: 电台, 1014: 视频
    /// offset: 起始点
    /// limit: 数量
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

    /// 搜索单曲
    /// keywords: 关键词
    /// offset: 起始点
    /// limit: 数量
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

    /// 搜索歌手
    /// keywords: 关键词
    /// offset: 起始点
    /// limit: 数量
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

    /// 搜索专辑
    /// keywords: 关键词
    /// offset: 起始点
    /// limit: 数量
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

    /// 搜索歌单
    /// keywords: 关键词
    /// offset: 起始点
    /// limit: 数量
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

    /// 搜索歌词
    /// keywords: 关键词
    /// offset: 起始点
    /// limit: 数量
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

    /// 获取歌手热门单曲
    /// id: 歌手 ID
    #[allow(unused)]
    pub async fn singer_songs(&self, id: u64) -> Result<Vec<SongInfo>> {
        let path = format!("/weapi/v1/artist/{}", id);
        let mut params = HashMap::new();
        let result = self
            .request(Method::Post, &path, params, CryptoApi::Weapi, "", false)
            .await?;
        to_song_info(result, Parse::Singer)
    }

    /// 获取歌手全部单曲
    /// id: 歌手 ID
    /// order: 排序方式:
    //	      "hot": 热门
    ///       "time": 时间
    /// offset: 起始点
    /// limit: 数量
    #[allow(unused)]
    pub async fn singer_all_songs(
        &self,
        id: u64,
        order: &str,
        offset: u16,
        limit: u16,
    ) -> Result<Vec<SongInfo>> {
        let path = "/weapi/v1/artist/songs";
        let mut params = HashMap::new();
        let id = id.to_string();
        let offset = offset.to_string();
        let limit = limit.to_string();
        params.insert("id", &id[..]);
        params.insert("private_cloud", "true");
        params.insert("work_type", "1");
        params.insert("order", order);
        params.insert("offset", &offset[..]);
        params.insert("limit", &limit[..]);
        let result = self
            .request(Method::Post, path, params, CryptoApi::Weapi, "", false)
            .await?;
        to_song_info(result, Parse::SingerSongs)
    }

    /// 全部新碟
    /// offset: 起始点
    /// limit: 数量
    /// area: ALL:全部,ZH:华语,EA:欧美,KR:韩国,JP:日本
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

    /// 专辑
    /// album_id: 专辑 id
    #[allow(unused)]
    pub async fn album(&self, album_id: u64) -> Result<AlbumDetail> {
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
        to_album_detail(&serde_json::from_str(&result)?)
    }

    /// 歌单动态信息
    /// songlist_id: 歌单 id
    #[allow(unused)]
    pub async fn songlist_detail_dynamic(&self, songlist_id: u64) -> Result<PlayListDetailDynamic> {
        let path = "/weapi/playlist/detail/dynamic";
        let mut params = HashMap::new();
        let id = songlist_id.to_string();
        params.insert("id", &id[..]);
        let result = self
            .request(Method::Post, path, params, CryptoApi::Weapi, "", true)
            .await?;
        to_songlist_detail_dynamic(result)
    }

    /// 专辑动态信息
    /// album_id: 专辑 id
    #[allow(unused)]
    pub async fn album_detail_dynamic(&self, album_id: u64) -> Result<AlbumDetailDynamic> {
        let path = "/weapi/album/detail/dynamic";
        let mut params = HashMap::new();
        let id = album_id.to_string();
        params.insert("id", &id[..]);
        let result = self
            .request(Method::Post, path, params, CryptoApi::Weapi, "", true)
            .await?;
        to_album_detail_dynamic(result)
    }

    /// 热门推荐歌单
    /// offset: 起始点
    /// limit: 数量
    /// order: 排序方式:
    //	      "hot": 热门，
    ///        "new": 最新
    /// cat: 全部,华语,欧美,日语,韩语,粤语,小语种,流行,摇滚,民谣,电子,舞曲,说唱,轻音乐,爵士,乡村,R&B/Soul,古典,民族,英伦,金属,朋克,蓝调,雷鬼,世界音乐,拉丁,另类/独立,New Age,古风,后摇,Bossa Nova,清晨,夜晚,学习,工作,午休,下午茶,地铁,驾车,运动,旅行,散步,酒吧,怀旧,清新,浪漫,性感,伤感,治愈,放松,孤独,感动,兴奋,快乐,安静,思念,影视原声,ACG,儿童,校园,游戏,70后,80后,90后,网络歌曲,KTV,经典,翻唱,吉他,钢琴,器乐,榜单,00后
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

    /// 精品歌单
    /// lasttime: 分页参数,取上一页最后一个歌单的 updateTime 获取下一页数据
    /// limit: 数量
    /// cat: 全部,华语,欧美,韩语,日语,粤语,小语种,运动,ACG,影视原声,流行,摇滚,后摇,古风,民谣,轻音乐,电子,器乐,说唱,古典,爵士
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

    /// 获取排行榜
    #[allow(unused)]
    pub async fn toplist(&self) -> Result<Vec<TopList>> {
        let path = "/api/toplist";
        let mut params = HashMap::new();
        let res = self
            .request(Method::Post, path, params, CryptoApi::Weapi, "", true)
            .await?;
        to_toplist(res)
    }

    /// 热门歌曲/排行榜
    /// list_id:
    /// 云音乐飙升榜: 19723756
    /// 云音乐新歌榜: 3779629
    /// 网易原创歌曲榜: 2884035
    /// 云音乐热歌榜: 3778678
    /// 云音乐古典音乐榜: 71384707
    /// 云音乐ACG音乐榜: 71385702
    /// 云音乐韩语榜: 745956260
    /// 云音乐国电榜: 10520166
    /// 云音乐嘻哈榜: 991319590']
    /// 抖音排行榜: 2250011882
    /// UK排行榜周榜: 180106
    /// 美国Billboard周榜: 60198
    /// KTV嗨榜: 21845217
    /// iTunes榜: 11641012
    /// Hit FM Top榜: 120001
    /// 日本Oricon周榜: 60131
    /// 台湾Hito排行榜: 112463
    /// 香港电台中文歌曲龙虎榜: 10169002
    /// 华语金曲榜: 4395559
    #[allow(unused)]
    pub async fn top_songs(&self, list_id: u64) -> Result<PlayListDetail> {
        self.song_list_detail(list_id).await
    }

    /// 查询歌词
    /// music_id: 歌曲id
    #[allow(unused)]
    pub async fn song_lyric(&self, music_id: u64) -> Result<Lyrics> {
        let empty = String::new();
        let csrf_token = self.csrf.get().unwrap_or(&empty);
        let path = "/weapi/song/lyric";
        let mut params = HashMap::new();
        let id = music_id.to_string();
        params.insert("id", &id[..]);
        params.insert("lv", "-1");
        params.insert("tv", "-1");
        params.insert("csrf_token", csrf_token);
        let result = self
            .request(Method::Post, path, params, CryptoApi::Weapi, "", true)
            .await?;
        to_lyric(result)
    }

    /// 收藏/取消收藏歌单
    /// like: true 收藏，false 取消
    /// id: 歌单 id
    #[allow(unused)]
    pub async fn song_list_like(&self, like: bool, id: u64) -> bool {
        let path = if like {
            "/api/playlist/subscribe"
        } else {
            "/api/playlist/unsubscribe"
        };
        let mut params = HashMap::new();
        let id = id.to_string();
        params.insert("id", &id[..]);
        // 收藏时需随请求体下发静态 checkToken，取消收藏则不带
        if like {
            params.insert("checkToken", CHECK_TOKEN);
        }
        // 易盾反作弊 token 每次实时获取，失败时不阻断请求
        let token = self.fetch_anti_cheat_token().await;
        let mut extra_headers = Vec::new();
        if let Some(token) = token.as_deref() {
            extra_headers.push((ANTI_CHEAT_HEADER, token));
        }
        if let Ok(result) = self
            .request_ext(
                Method::Post,
                path,
                params,
                CryptoApi::Eapi,
                "",
                true,
                &extra_headers,
            )
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

    /// 收藏/取消收藏专辑
    /// like: true 收藏，false 取消
    /// id: 歌单 id
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

    /// 获取 APP 首页信息
    #[allow(unused)]
    pub async fn homepage(&self, client_type: ClientType) -> Result<String> {
        let path = "/api/homepage/block/page";
        let mut params = HashMap::new();
        params.insert("refresh", "false");
        params.insert("cursor", "null");
        self.request(Method::Post, path, params, CryptoApi::Weapi, "", true)
            .await
    }

    /// 获取首页轮播图
    #[allow(unused)]
    pub async fn banners(&self) -> Result<Vec<BannersInfo>> {
        let path = "/weapi/v2/banner/get";
        let mut params = HashMap::new();
        params.insert("clientType", "pc");
        let result = self
            .request(Method::Post, path, params, CryptoApi::Weapi, "", true)
            .await?;
        to_banners_info(result)
    }

    /// 从网络下载图片
    /// url: 网址
    /// path: 本地保存路径(包含文件名)
    /// width: 宽度
    /// high: 高度
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

    /// 从网络下载音乐
    /// url: 网址
    /// path: 本地保存路径(包含文件名)
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

    /// 用户电台定阅列表
    /// offset: 列表起点号
    /// limit: 列表长度
    #[allow(unused)]
    pub async fn user_radio_sublist(&self, offset: u16, limit: u16) -> Result<Vec<SongList>> {
        let path = "/weapi/djradio/get/subed";
        let mut params = HashMap::new();
        let offset = offset.to_string();
        let limit = limit.to_string();
        params.insert("total", "true");
        params.insert("offset", offset.as_str());
        params.insert("limit", limit.as_str());
        let result = self
            .request(Method::Post, path, params, CryptoApi::Weapi, "", true)
            .await?;
        to_song_list(result, Parse::Radio)
    }

    /// 电台节目列表
    /// rid: 电台ID
    /// offset: 列表起点号
    /// limit: 列表长度
    #[allow(unused)]
    pub async fn radio_program(&self, rid: u64, offset: u16, limit: u16) -> Result<Vec<SongInfo>> {
        let path = "/weapi/dj/program/byradio";
        let mut params = HashMap::new();
        let id = rid.to_string();
        let offset = offset.to_string();
        let limit = limit.to_string();
        params.insert("radioId", id.as_str());
        params.insert("offset", offset.as_str());
        params.insert("limit", limit.as_str());
        params.insert("asc", "false");
        let result = self
            .request(Method::Post, path, params, CryptoApi::Weapi, "", true)
            .await?;
        to_song_info(result, Parse::Radio)
    }

    /// 心动模式/智能播放
    /// song_id: 歌曲ID
    /// playlist_id: 歌单ID
    #[allow(unused)]
    pub async fn playmode_intelligence_list(&self, sid: u64, pid: u64) -> Result<Vec<SongInfo>> {
        let path = "/weapi/playmode/intelligence/list";
        let mut params = HashMap::new();
        let id = sid.to_string();
        let pid = pid.to_string();
        params.insert("songId", id.as_str());
        params.insert("type", "fromPlayOne");
        params.insert("playlistId", pid.as_str());
        params.insert("startMusicId", id.as_str());
        params.insert("count", "1");
        let result = self
            .request(Method::Post, path, params, CryptoApi::Weapi, "", true)
            .await?;
        to_song_info(result, Parse::Intelligence)
    }
}

/// 解析易盾 v3 接口返回的 JSONP，取出反作弊 token
/// 正常响应形如: null([200,1793073626061,"s0qdf2yP98JEEVUEEQKIE2+b0CVOmrX2"])
fn parse_anti_cheat_token(body: &str) -> Option<String> {
    let caps = _CHECK_TOKEN.captures(body)?;
    if caps.get(1)?.as_str() != "200" {
        return None;
    }
    let token = caps.get(2)?.as_str();
    if token.is_empty() {
        None
    } else {
        Some(token.to_string())
    }
}

fn build_cookie(cryptoapi: &CryptoApi) -> String {
    let (os, appver, osver) = match cryptoapi {
        CryptoApi::Eapi => ("iphone", "9.0.90", "16.2"),
        CryptoApi::Xeapi => ("android", "9.5.61", "16"),
        _ => ("pc", "2.7.1.198277", "10"),
    };
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let now_str = now.to_string();

    let nuid = format!("{:x}{:x}", rand::random::<u64>(), rand::random::<u64>());
    let nnid = format!("{},{}", nuid, now_str);
    let nmtid = format!("{:x}", rand::random::<u64>());
    let wnmcid = format!(
        "{:02x}{:02x}{:02x}.{}",
        rand::random::<u8>(),
        rand::random::<u8>(),
        rand::random::<u8>(),
        now_str,
    );

    format!(
        "os={}; appver={}; osver={}; deviceId=; WEVNSM=1.0.0; \
         WNMCID={}; _ntes_nnid={}; _ntes_nuid={}; NMTID={}; \
         __remember_me=true; channel=",
        os, appver, osver, wnmcid, nnid, nuid, nmtid,
    )
}

fn choose_user_agent(ua: &str) -> &str {
    let index = if ua == "mobile" {
        rand::random::<u16>() % 7
    } else if ua == "pc" {
        rand::random::<u16>() % 5 + 8
    } else if !ua.is_empty() {
        return ua;
    } else {
        rand::random::<u16>() % USER_AGENT_LIST.len() as u16
    };
    USER_AGENT_LIST[index as usize]
}

// 服务端 playlist.tracks 最多返回约 1000 条完整歌曲，
// 用 trackIds 补齐缺失部分并保持歌单顺序
async fn complete_playlist<F, Fut>(json: &serde_json::Value, mut fetch: F) -> Result<PlayListDetail>
where
    F: FnMut(Vec<u64>) -> Fut,
    Fut: std::future::Future<Output = Result<Vec<SongInfo>>>,
{
    let mut detail = to_mix_detail(json)?;
    let Some(track_ids) = json["playlist"]["trackIds"].as_array() else {
        return Ok(detail);
    };
    let ids: Vec<u64> = track_ids
        .iter()
        .map(|t| {
            t["id"]
                .as_u64()
                .ok_or_else(|| anyhow!("Invalid playlist track ID"))
        })
        .collect::<Result<Vec<_>>>()?;
    let mut songs: HashMap<u64, SongInfo> = detail
        .songs
        .iter()
        .cloned()
        .map(|song| (song.id, song))
        .collect();
    let missing: Vec<u64> = ids
        .iter()
        .copied()
        .filter(|id| !songs.contains_key(id))
        .collect();
    if missing.is_empty() {
        return Ok(detail);
    }
    for batch in missing.chunks(500) {
        for song in fetch(batch.to_vec()).await? {
            songs.insert(song.id, song);
        }
    }
    // 已删除/失效的曲目可能不在歌曲详情返回中，跳过以保持顺序
    detail.songs = ids.iter().filter_map(|id| songs.get(id).cloned()).collect();
    Ok(detail)
}

#[cfg(test)]
mod tests {

    use super::*;

    fn playlist_fixture(total: u64, loaded: u64) -> serde_json::Value {
        serde_json::json!({
            "code": 200,
            "playlist": {
                "id": 42, "name": "test", "coverImgUrl": "", "description": "",
                "createTime": 0, "trackUpdateTime": 0,
                "trackIds": (1..=total).map(|id| serde_json::json!({"id": id})).collect::<Vec<_>>(),
                "tracks": (1..=loaded).map(|id| serde_json::json!({
                    "id": id, "name": "song", "ar": [{"name": "artist"}],
                    "al": {"id": 1, "name": "album", "picUrl": ""}, "dt": 1000
                })).collect::<Vec<_>>()
            },
            "privileges": (1..=loaded).map(|_| serde_json::json!({"st": 0, "fee": 0})).collect::<Vec<_>>()
        })
    }

    #[async_std::test]
    async fn playlist_completes_batches_in_original_order() {
        let json = playlist_fixture(2201, 1000);
        let template = to_mix_detail(&json).unwrap().songs[0].clone();
        let mut requests = Vec::new();
        let detail = complete_playlist(&json, |ids| {
            requests.push(ids.clone());
            let songs = ids
                .into_iter()
                .map(|id| SongInfo {
                    id,
                    ..template.clone()
                })
                .collect();
            std::future::ready(Ok(songs))
        })
        .await
        .unwrap();
        assert_eq!(
            requests.iter().map(Vec::len).collect::<Vec<_>>(),
            [500, 500, 201]
        );
        assert_eq!(requests.concat(), (1001..=2201).collect::<Vec<_>>());
        assert_eq!(
            detail.songs.iter().map(|s| s.id).collect::<Vec<_>>(),
            (1..=2201).collect::<Vec<_>>()
        );
    }

    #[async_std::test]
    async fn playlist_keeps_small_and_legacy_responses() {
        for count in [0u64, 2, 1000] {
            let mut json = playlist_fixture(count, count);
            for legacy in [false, true] {
                if legacy {
                    json["playlist"].as_object_mut().unwrap().remove("trackIds");
                }
                let detail = complete_playlist(&json, |_| {
                    std::future::ready(Err(anyhow!("unexpected extra request")))
                })
                .await
                .unwrap();
                assert_eq!(detail.songs.len(), count as usize);
            }
        }
    }

    #[async_std::test]
    async fn playlist_propagates_batch_failure() {
        let result = complete_playlist(&playlist_fixture(1001, 1000), |_| {
            std::future::ready(Err(anyhow!("batch failed")))
        })
        .await;
        assert_eq!(result.unwrap_err().to_string(), "batch failed");
    }

    #[async_std::test]
    async fn playlist_handles_omitted_tracks_without_reordering() {
        let json = playlist_fixture(4, 2);
        let template = to_mix_detail(&json).unwrap().songs[0].clone();
        let detail = complete_playlist(&json, |_| {
            std::future::ready(Ok(vec![SongInfo {
                id: 4,
                ..template.clone()
            }]))
        })
        .await
        .unwrap();
        assert_eq!(
            detail.songs.iter().map(|s| s.id).collect::<Vec<_>>(),
            [1, 2, 4]
        );
    }

    #[test]
    fn song_details_allow_an_empty_batch() {
        let songs = to_song_info(r#"{"code":200,"songs":[]}"#.into(), Parse::Usl).unwrap();
        assert!(songs.is_empty());
    }

    #[test]
    fn anti_cheat_token_parses_dun_jsonp() {
        assert_eq!(
            parse_anti_cheat_token(r#"null([200,1793073626061,"s0qdf2yP98JEEVUEEQKIE2+b0CVOmrX2"])"#)
                .as_deref(),
            Some("s0qdf2yP98JEEVUEEQKIE2+b0CVOmrX2")
        );
        // 非 200 状态、空 token、非 JSONP 响应都不应产出 token
        assert!(parse_anti_cheat_token(r#"null([500,0,""])"#).is_none());
        assert!(parse_anti_cheat_token(r#"null([200,0,""])"#).is_none());
        assert!(parse_anti_cheat_token("").is_none());
    }

    #[test]
    fn playlist_subscribe_payload_carries_check_token() {
        let mut params = HashMap::new();
        params.insert("id", "106697785");
        params.insert("checkToken", CHECK_TOKEN);
        let data = serde_json::json!(params);
        let encrypted = Crypto::eapi("/api/playlist/subscribe", &data.to_string());
        // QueryParams::stringify 输出形如 params=<HEX>&，需去掉前后缀再解码
        let hex = encrypted
            .strip_prefix("params=")
            .expect("eapi 输出应为 params=... 形式")
            .trim_end_matches('&');
        let (url, payload) = Crypto::eapi_req_decrypt(hex).unwrap();
        assert_eq!(url, "/api/playlist/subscribe");
        assert!(payload.contains(CHECK_TOKEN));
        assert!(payload.contains(r#""id":"106697785""#));
    }

    #[async_std::test]
    #[ignore = "requires access to NetEase and a public playlist with over 1000 tracks"]
    async fn playlist_live_over_1000() {
        let api = MusicApi::default();
        let detail = api.song_list_detail(2092244409).await.unwrap();
        assert!(detail.songs.len() > 1000);
    }

    #[async_std::test]
    async fn test() {
        let api = MusicApi::default();
        assert!(api.songs_url(&[1908049566], "320000").await.is_ok());
        assert!(api
            .songs_url_v1(&[1908049566], SongQuality::Standard)
            .await
            .is_ok());
        dbg!(api.recommend_songs().await.unwrap());
        let pld = match api.song_list_detail(2092244409).await {
            Ok(p) => p,
            Err(e) => panic!("song_list_detail failed: {e:#}"),
        };
        dbg!(pld.songs.len());

        assert!(api.banners().await.is_ok());
    }

    fn require_sync<T: Sync>(_: T) {}

    #[async_std::test]
    async fn music_api_send() {
        let api = MusicApi::default();
        require_sync(api);
    }
}
