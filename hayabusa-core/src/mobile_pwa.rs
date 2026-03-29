//! Mobile PWA Enhancements for Hayabusa.
//!
//! Native app-like features for Progressive Web Apps:
//! - Push notifications (Web Push API)
//! - Background sync
//! - iOS-specific meta tags (splash screen, status bar)
//! - Add-to-homescreen prompt handling
//! - Touch gesture support
//! - Screen orientation lock
//! - Share Target API
//! - Badging API
//! - App shell pattern
//!
//! ## Usage
//! ```ignore
//! let pwa = MobilePwaConfig::new("My App")
//!     .theme_color("#1a1a2e")
//!     .ios_splash("/splash-1125x2436.png", 1125, 2436)
//!     .orientation(ScreenOrientation::Portrait)
//!     .share_target("/share", &["title", "text", "url"])
//!     .push_notifications("/api/push/subscribe");
//! ```

use std::collections::HashMap;

/// Comprehensive mobile PWA configuration
#[derive(Debug, Clone)]
pub struct MobilePwaConfig {
    /// App name
    pub name: String,
    /// Short name for homescreen
    pub short_name: String,
    /// Theme color
    pub theme_color: String,
    /// Background color for splash screen
    pub background_color: String,
    /// Display mode
    pub display: MobileDisplay,
    /// Screen orientation
    pub orientation: ScreenOrientation,
    /// iOS splash screens
    pub ios_splashes: Vec<IosSplash>,
    /// iOS status bar style
    pub ios_status_bar: IosStatusBar,
    /// Share target configuration
    pub share_target: Option<ShareTargetConfig>,
    /// Push notification endpoint
    pub push_endpoint: Option<String>,
    /// VAPID public key for push
    pub vapid_public_key: Option<String>,
    /// Enable background sync
    pub background_sync: bool,
    /// App shortcuts
    pub shortcuts: Vec<AppShortcut>,
    /// Related native apps
    pub related_apps: Vec<RelatedApp>,
    /// App categories
    pub categories: Vec<String>,
    /// Enable badging API
    pub enable_badging: bool,
    /// Scope
    pub scope: String,
    /// Start URL
    pub start_url: String,
}

#[derive(Debug, Clone)]
pub enum MobileDisplay {
    Standalone,
    Fullscreen,
    MinimalUi,
}

impl MobileDisplay {
    fn as_str(&self) -> &str {
        match self {
            MobileDisplay::Standalone => "standalone",
            MobileDisplay::Fullscreen => "fullscreen",
            MobileDisplay::MinimalUi => "minimal-ui",
        }
    }
}

#[derive(Debug, Clone)]
pub enum ScreenOrientation {
    Any,
    Portrait,
    Landscape,
    PortraitPrimary,
    LandscapePrimary,
}

impl ScreenOrientation {
    fn as_str(&self) -> &str {
        match self {
            ScreenOrientation::Any => "any",
            ScreenOrientation::Portrait => "portrait",
            ScreenOrientation::Landscape => "landscape",
            ScreenOrientation::PortraitPrimary => "portrait-primary",
            ScreenOrientation::LandscapePrimary => "landscape-primary",
        }
    }
}

#[derive(Debug, Clone)]
pub enum IosStatusBar {
    Default,
    Black,
    BlackTranslucent,
}

impl IosStatusBar {
    fn as_str(&self) -> &str {
        match self {
            IosStatusBar::Default => "default",
            IosStatusBar::Black => "black",
            IosStatusBar::BlackTranslucent => "black-translucent",
        }
    }
}

/// iOS splash screen definition
#[derive(Debug, Clone)]
pub struct IosSplash {
    pub href: String,
    pub width: u32,
    pub height: u32,
    pub pixel_ratio: u32,
}

/// Share Target API configuration
#[derive(Debug, Clone)]
pub struct ShareTargetConfig {
    pub action: String,
    pub method: String,
    pub enctype: String,
    pub params: ShareTargetParams,
}

#[derive(Debug, Clone)]
pub struct ShareTargetParams {
    pub title: Option<String>,
    pub text: Option<String>,
    pub url: Option<String>,
    pub files: Vec<ShareTargetFile>,
}

#[derive(Debug, Clone)]
pub struct ShareTargetFile {
    pub name: String,
    pub accept: Vec<String>,
}

/// App shortcut (long-press menu on homescreen icon)
#[derive(Debug, Clone)]
pub struct AppShortcut {
    pub name: String,
    pub short_name: Option<String>,
    pub url: String,
    pub icon: Option<String>,
}

/// Related native app
#[derive(Debug, Clone)]
pub struct RelatedApp {
    pub platform: String,
    pub url: String,
    pub id: Option<String>,
}

impl MobilePwaConfig {
    pub fn new(name: impl Into<String>) -> Self {
        let name = name.into();
        let short_name = name.clone();
        Self {
            name,
            short_name,
            theme_color: "#000000".to_string(),
            background_color: "#ffffff".to_string(),
            display: MobileDisplay::Standalone,
            orientation: ScreenOrientation::Any,
            ios_splashes: Vec::new(),
            ios_status_bar: IosStatusBar::Default,
            share_target: None,
            push_endpoint: None,
            vapid_public_key: None,
            background_sync: false,
            shortcuts: Vec::new(),
            related_apps: Vec::new(),
            categories: Vec::new(),
            enable_badging: false,
            scope: "/".to_string(),
            start_url: "/".to_string(),
        }
    }

    pub fn short_name(mut self, name: impl Into<String>) -> Self {
        self.short_name = name.into();
        self
    }

    pub fn theme_color(mut self, color: impl Into<String>) -> Self {
        self.theme_color = color.into();
        self
    }

    pub fn background_color(mut self, color: impl Into<String>) -> Self {
        self.background_color = color.into();
        self
    }

    pub fn display(mut self, display: MobileDisplay) -> Self {
        self.display = display;
        self
    }

    pub fn orientation(mut self, orientation: ScreenOrientation) -> Self {
        self.orientation = orientation;
        self
    }

    pub fn ios_status_bar(mut self, style: IosStatusBar) -> Self {
        self.ios_status_bar = style;
        self
    }

    /// Add an iOS splash screen
    pub fn ios_splash(
        mut self,
        href: impl Into<String>,
        width: u32,
        height: u32,
    ) -> Self {
        self.ios_splashes.push(IosSplash {
            href: href.into(),
            width,
            height,
            pixel_ratio: if width > 1500 { 3 } else { 2 },
        });
        self
    }

    /// Common iOS splash screens for all modern devices
    pub fn ios_splash_all(self, base_path: &str) -> Self {
        self.ios_splash(format!("{}/splash-1170x2532.png", base_path), 1170, 2532) // iPhone 12/13/14
            .ios_splash(format!("{}/splash-1179x2556.png", base_path), 1179, 2556) // iPhone 14 Pro
            .ios_splash(format!("{}/splash-1290x2796.png", base_path), 1290, 2796) // iPhone 14 Pro Max
            .ios_splash(format!("{}/splash-1125x2436.png", base_path), 1125, 2436) // iPhone X/XS/11 Pro
            .ios_splash(format!("{}/splash-1242x2688.png", base_path), 1242, 2688) // iPhone XS Max/11 Pro Max
            .ios_splash(format!("{}/splash-828x1792.png", base_path), 828, 1792)   // iPhone XR/11
            .ios_splash(format!("{}/splash-1668x2388.png", base_path), 1668, 2388) // iPad Pro 11
            .ios_splash(format!("{}/splash-2048x2732.png", base_path), 2048, 2732) // iPad Pro 12.9
    }

    /// Configure Share Target API
    pub fn share_target(
        mut self,
        action: impl Into<String>,
        params: &[&str],
    ) -> Self {
        let mut sp = ShareTargetParams {
            title: None,
            text: None,
            url: None,
            files: Vec::new(),
        };
        for &p in params {
            match p {
                "title" => sp.title = Some("title".to_string()),
                "text" => sp.text = Some("text".to_string()),
                "url" => sp.url = Some("url".to_string()),
                _ => {}
            }
        }
        self.share_target = Some(ShareTargetConfig {
            action: action.into(),
            method: "POST".to_string(),
            enctype: "multipart/form-data".to_string(),
            params: sp,
        });
        self
    }

    /// Configure Share Target with file support
    pub fn share_target_files(
        mut self,
        action: impl Into<String>,
        file_name: &str,
        accept: &[&str],
    ) -> Self {
        // Create or update share target
        let st = self.share_target.get_or_insert(ShareTargetConfig {
            action: action.into(),
            method: "POST".to_string(),
            enctype: "multipart/form-data".to_string(),
            params: ShareTargetParams {
                title: Some("title".to_string()),
                text: Some("text".to_string()),
                url: Some("url".to_string()),
                files: Vec::new(),
            },
        });
        st.params.files.push(ShareTargetFile {
            name: file_name.to_string(),
            accept: accept.iter().map(|s| s.to_string()).collect(),
        });
        self
    }

    /// Enable push notifications
    pub fn push_notifications(
        mut self,
        endpoint: impl Into<String>,
    ) -> Self {
        self.push_endpoint = Some(endpoint.into());
        self
    }

    /// Set VAPID public key for push
    pub fn vapid_key(mut self, key: impl Into<String>) -> Self {
        self.vapid_public_key = Some(key.into());
        self
    }

    /// Enable background sync
    pub fn background_sync(mut self) -> Self {
        self.background_sync = true;
        self
    }

    /// Add an app shortcut
    pub fn shortcut(
        mut self,
        name: impl Into<String>,
        url: impl Into<String>,
    ) -> Self {
        self.shortcuts.push(AppShortcut {
            name: name.into(),
            short_name: None,
            url: url.into(),
            icon: None,
        });
        self
    }

    /// Add an app shortcut with icon
    pub fn shortcut_with_icon(
        mut self,
        name: impl Into<String>,
        url: impl Into<String>,
        icon: impl Into<String>,
    ) -> Self {
        self.shortcuts.push(AppShortcut {
            name: name.into(),
            short_name: None,
            url: url.into(),
            icon: Some(icon.into()),
        });
        self
    }

    /// Enable badging API
    pub fn badging(mut self) -> Self {
        self.enable_badging = true;
        self
    }

    /// Add app category
    pub fn category(mut self, cat: impl Into<String>) -> Self {
        self.categories.push(cat.into());
        self
    }

    /// Add a related native app
    pub fn related_app(
        mut self,
        platform: impl Into<String>,
        url: impl Into<String>,
    ) -> Self {
        self.related_apps.push(RelatedApp {
            platform: platform.into(),
            url: url.into(),
            id: None,
        });
        self
    }

    // ───────────────────────────────────────────
    //  Rendering
    // ───────────────────────────────────────────

    /// Render all HTML meta tags for <head>
    pub fn render_meta_tags(&self) -> String {
        let mut html = String::new();

        // Standard PWA meta
        html.push_str(&format!(
            "<meta name=\"theme-color\" content=\"{}\" />\n",
            self.theme_color
        ));
        html.push_str("<meta name=\"mobile-web-app-capable\" content=\"yes\" />\n");
        html.push_str(&format!(
            "<meta name=\"viewport\" content=\"width=device-width,initial-scale=1,viewport-fit=cover\" />\n"
        ));

        // iOS meta tags
        html.push_str("<meta name=\"apple-mobile-web-app-capable\" content=\"yes\" />\n");
        html.push_str(&format!(
            "<meta name=\"apple-mobile-web-app-status-bar-style\" content=\"{}\" />\n",
            self.ios_status_bar.as_str()
        ));
        html.push_str(&format!(
            "<meta name=\"apple-mobile-web-app-title\" content=\"{}\" />\n",
            self.short_name
        ));

        // iOS splash screens
        for splash in &self.ios_splashes {
            let device_w = splash.width / splash.pixel_ratio;
            let device_h = splash.height / splash.pixel_ratio;
            html.push_str(&format!(
                "<link rel=\"apple-touch-startup-image\" href=\"{}\" media=\"(device-width: {}px) and (device-height: {}px) and (-webkit-device-pixel-ratio: {})\" />\n",
                splash.href, device_w, device_h, splash.pixel_ratio
            ));
        }

        // Manifest link
        html.push_str("<link rel=\"manifest\" href=\"/manifest.json\" />\n");

        html
    }

    /// Generate the extended manifest.json
    pub fn render_manifest(&self) -> String {
        let mut m = serde_json::Map::new();
        m.insert("name".into(), serde_json::Value::String(self.name.clone()));
        m.insert("short_name".into(), serde_json::Value::String(self.short_name.clone()));
        m.insert("start_url".into(), serde_json::Value::String(self.start_url.clone()));
        m.insert("scope".into(), serde_json::Value::String(self.scope.clone()));
        m.insert("display".into(), serde_json::Value::String(self.display.as_str().to_string()));
        m.insert("orientation".into(), serde_json::Value::String(self.orientation.as_str().to_string()));
        m.insert("theme_color".into(), serde_json::Value::String(self.theme_color.clone()));
        m.insert("background_color".into(), serde_json::Value::String(self.background_color.clone()));

        // Categories
        if !self.categories.is_empty() {
            m.insert("categories".into(), serde_json::Value::Array(
                self.categories.iter().map(|c| serde_json::Value::String(c.clone())).collect()
            ));
        }

        // Shortcuts
        if !self.shortcuts.is_empty() {
            let shortcuts: Vec<serde_json::Value> = self.shortcuts.iter().map(|s| {
                let mut sc = serde_json::Map::new();
                sc.insert("name".into(), serde_json::Value::String(s.name.clone()));
                sc.insert("url".into(), serde_json::Value::String(s.url.clone()));
                if let Some(ref short) = s.short_name {
                    sc.insert("short_name".into(), serde_json::Value::String(short.clone()));
                }
                if let Some(ref icon) = s.icon {
                    sc.insert("icons".into(), serde_json::json!([{"src": icon, "sizes": "96x96"}]));
                }
                serde_json::Value::Object(sc)
            }).collect();
            m.insert("shortcuts".into(), serde_json::Value::Array(shortcuts));
        }

        // Share target
        if let Some(ref st) = self.share_target {
            let mut target = serde_json::Map::new();
            target.insert("action".into(), serde_json::Value::String(st.action.clone()));
            target.insert("method".into(), serde_json::Value::String(st.method.clone()));
            target.insert("enctype".into(), serde_json::Value::String(st.enctype.clone()));

            let mut params = serde_json::Map::new();
            if let Some(ref t) = st.params.title {
                params.insert("title".into(), serde_json::Value::String(t.clone()));
            }
            if let Some(ref t) = st.params.text {
                params.insert("text".into(), serde_json::Value::String(t.clone()));
            }
            if let Some(ref u) = st.params.url {
                params.insert("url".into(), serde_json::Value::String(u.clone()));
            }
            if !st.params.files.is_empty() {
                let files: Vec<serde_json::Value> = st.params.files.iter().map(|f| {
                    serde_json::json!({
                        "name": f.name,
                        "accept": f.accept
                    })
                }).collect();
                params.insert("files".into(), serde_json::Value::Array(files));
            }
            target.insert("params".into(), serde_json::Value::Object(params));
            m.insert("share_target".into(), serde_json::Value::Object(target));
        }

        // Related apps
        if !self.related_apps.is_empty() {
            let apps: Vec<serde_json::Value> = self.related_apps.iter().map(|a| {
                let mut app = serde_json::Map::new();
                app.insert("platform".into(), serde_json::Value::String(a.platform.clone()));
                app.insert("url".into(), serde_json::Value::String(a.url.clone()));
                if let Some(ref id) = a.id {
                    app.insert("id".into(), serde_json::Value::String(id.clone()));
                }
                serde_json::Value::Object(app)
            }).collect();
            m.insert("related_applications".into(), serde_json::Value::Array(apps));
        }

        serde_json::to_string_pretty(&serde_json::Value::Object(m)).unwrap_or_default()
    }

    /// Generate push notification subscription script
    pub fn render_push_script(&self) -> String {
        let Some(ref endpoint) = self.push_endpoint else {
            return String::new();
        };

        let vapid_key = self.vapid_public_key.as_deref().unwrap_or("");

        format!(
            r#"<script>
(function(){{
if(!('serviceWorker' in navigator)||!('PushManager' in window))return;
navigator.serviceWorker.ready.then(function(reg){{
return reg.pushManager.getSubscription().then(function(sub){{
if(sub)return sub;
var key=Uint8Array.from(atob('{vapid_key}'.replace(/-/g,'+').replace(/_/g,'/')),function(c){{return c.charCodeAt(0)}});
return reg.pushManager.subscribe({{userVisibleOnly:true,applicationServerKey:key}});
}});
}}).then(function(sub){{
if(sub)fetch('{endpoint}',{{method:'POST',headers:{{'Content-Type':'application/json'}},body:JSON.stringify(sub)}});
}}).catch(function(e){{console.log('[Hayabusa] Push subscription failed:',e)}});
}})();
</script>"#,
            vapid_key = vapid_key,
            endpoint = endpoint,
        )
    }

    /// Generate background sync registration script
    pub fn render_background_sync_script(&self) -> String {
        if !self.background_sync {
            return String::new();
        }

        r#"<script>
(function(){
if(!('serviceWorker' in navigator)||!('SyncManager' in window))return;
// Queue outbox items when offline
window.__hayabusaSync={
queue:function(tag,data){
var store=JSON.parse(localStorage.getItem('__hayabusa_sync')||'[]');
store.push({tag:tag,data:data,ts:Date.now()});
localStorage.setItem('__hayabusa_sync',JSON.stringify(store));
if(navigator.serviceWorker.controller){
navigator.serviceWorker.ready.then(function(reg){reg.sync.register(tag)});
}
}
};
})();
</script>"#
            .to_string()
    }

    /// Generate add-to-homescreen prompt handler
    pub fn render_install_prompt_script(&self) -> String {
        r#"<script>
(function(){
var deferredPrompt;
window.addEventListener('beforeinstallprompt',function(e){
e.preventDefault();
deferredPrompt=e;
document.dispatchEvent(new CustomEvent('hayabusa:installable'));
});
window.__hayabusaInstall=function(){
if(!deferredPrompt)return Promise.reject('No prompt');
deferredPrompt.prompt();
return deferredPrompt.userChoice.then(function(r){
deferredPrompt=null;
return r.outcome;
});
};
window.addEventListener('appinstalled',function(){
document.dispatchEvent(new CustomEvent('hayabusa:installed'));
});
})();
</script>"#
            .to_string()
    }

    /// Generate badging API helpers
    pub fn render_badging_script(&self) -> String {
        if !self.enable_badging {
            return String::new();
        }

        r#"<script>
window.__hayabusaBadge={
set:function(count){if('setAppBadge' in navigator)navigator.setAppBadge(count)},
clear:function(){if('clearAppBadge' in navigator)navigator.clearAppBadge()}
};
</script>"#
            .to_string()
    }

    /// Render all scripts needed for the mobile PWA
    pub fn render_all_scripts(&self) -> String {
        let mut scripts = String::new();
        scripts.push_str(&self.render_install_prompt_script());
        scripts.push_str(&self.render_push_script());
        scripts.push_str(&self.render_background_sync_script());
        scripts.push_str(&self.render_badging_script());
        scripts
    }
}

// ─────────────────────────────────────────────
//  Touch Gesture Support
// ─────────────────────────────────────────────

/// Touch gesture configuration for mobile-native feel
#[derive(Debug, Clone, Default)]
pub struct TouchGestureConfig {
    /// Enable swipe-back navigation
    pub swipe_back: bool,
    /// Enable pull-to-refresh
    pub pull_to_refresh: bool,
    /// Pull-to-refresh endpoint
    pub refresh_endpoint: Option<String>,
    /// Swipe threshold in pixels
    pub swipe_threshold: u32,
}

impl TouchGestureConfig {
    pub fn new() -> Self {
        Self {
            swipe_back: false,
            pull_to_refresh: false,
            refresh_endpoint: None,
            swipe_threshold: 80,
        }
    }

    /// Enable swipe-back navigation (swipe right to go back)
    pub fn enable_swipe_back(mut self) -> Self {
        self.swipe_back = true;
        self
    }

    /// Enable pull-to-refresh
    pub fn enable_pull_to_refresh(mut self, endpoint: Option<&str>) -> Self {
        self.pull_to_refresh = true;
        self.refresh_endpoint = endpoint.map(String::from);
        self
    }

    /// Set swipe threshold
    pub fn threshold(mut self, px: u32) -> Self {
        self.swipe_threshold = px;
        self
    }

    /// Generate the touch gesture JavaScript
    pub fn render_script(&self) -> String {
        let mut script = String::from("<script>\n(function(){\n");

        // Swipe-back navigation
        if self.swipe_back {
            script.push_str(&format!(
                r#"var sx=0,sy=0,threshold={threshold};
document.addEventListener('touchstart',function(e){{sx=e.touches[0].clientX;sy=e.touches[0].clientY}},{{passive:true}});
document.addEventListener('touchend',function(e){{
var dx=e.changedTouches[0].clientX-sx;
var dy=e.changedTouches[0].clientY-sy;
if(dx>threshold&&Math.abs(dy)<threshold/2&&sx<50)history.back();
}},{{passive:true}});
"#,
                threshold = self.swipe_threshold
            ));
        }

        // Pull-to-refresh
        if self.pull_to_refresh {
            let refresh_action = if let Some(ref ep) = self.refresh_endpoint {
                format!(
                    "fetch('{}').then(function(){{location.reload()}})",
                    ep
                )
            } else {
                "location.reload()".to_string()
            };

            script.push_str(&format!(
                r#"var pullStart=0,pulling=false;
var pullEl=document.createElement('div');
pullEl.style.cssText='position:fixed;top:-60px;left:50%;transform:translateX(-50%);width:40px;height:40px;border-radius:50%;background:#333;z-index:99999;transition:top 0.2s;display:flex;align-items:center;justify-content:center;color:#fff;font-size:1.2rem';
pullEl.textContent='↓';
document.body.appendChild(pullEl);
document.addEventListener('touchstart',function(e){{
if(window.scrollY===0){{pullStart=e.touches[0].clientY;pulling=true}}
}},{{passive:true}});
document.addEventListener('touchmove',function(e){{
if(!pulling)return;
var dy=e.touches[0].clientY-pullStart;
if(dy>0&&dy<150){{pullEl.style.top=(dy-60)+'px'}}
}},{{passive:true}});
document.addEventListener('touchend',function(e){{
if(!pulling)return;pulling=false;
var dy=e.changedTouches[0].clientY-pullStart;
pullEl.style.top='-60px';
if(dy>{threshold}){{{refresh_action}}}
}},{{passive:true}});
"#,
                threshold = self.swipe_threshold,
                refresh_action = refresh_action
            ));
        }

        script.push_str("})();\n</script>");
        script
    }
}

/// Generate CSS for safe area insets (notch handling)
pub fn safe_area_css() -> &'static str {
    "body{padding-top:env(safe-area-inset-top);padding-bottom:env(safe-area-inset-bottom);padding-left:env(safe-area-inset-left);padding-right:env(safe-area-inset-right)}"
}

/// Generate CSS for mobile-optimized touch targets
pub fn touch_target_css() -> &'static str {
    "a,button,input,select,textarea,[role=button]{min-height:44px;min-width:44px}input,textarea{font-size:16px}*{-webkit-tap-highlight-color:transparent}html{-webkit-text-size-adjust:100%}"
}

/// Generate the service worker additions for push and background sync
pub fn mobile_sw_additions() -> String {
    r#"
// Push notification handler
self.addEventListener('push',function(e){
var data=e.data?e.data.json():{title:'Notification',body:''};
e.waitUntil(self.registration.showNotification(data.title,{
body:data.body,icon:data.icon||'/icon-192.png',
badge:data.badge||'/badge-72.png',
data:data.data||{},
actions:data.actions||[]
}));
});

// Notification click handler
self.addEventListener('notificationclick',function(e){
e.notification.close();
var url=e.notification.data.url||'/';
e.waitUntil(clients.matchAll({type:'window'}).then(function(cs){
for(var i=0;i<cs.length;i++){if(cs[i].url===url&&'focus' in cs[i])return cs[i].focus()}
return clients.openWindow(url);
}));
});

// Background sync handler
self.addEventListener('sync',function(e){
e.waitUntil(
caches.open('__hayabusa_outbox').then(function(cache){
return cache.keys().then(function(keys){
return Promise.all(keys.map(function(req){
return cache.match(req).then(function(res){
return res.json().then(function(data){
return fetch(data.url,{method:data.method,headers:data.headers,body:data.body})
.then(function(){return cache.delete(req)});
});
});
}));
});
})
);
});
"#
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_config() {
        let pwa = MobilePwaConfig::new("Test App")
            .theme_color("#ff0000")
            .short_name("Test");

        let meta = pwa.render_meta_tags();
        assert!(meta.contains("theme-color"));
        assert!(meta.contains("#ff0000"));
        assert!(meta.contains("apple-mobile-web-app-capable"));
        assert!(meta.contains("apple-mobile-web-app-title"));
        assert!(meta.contains("Test"));
        assert!(meta.contains("viewport-fit=cover"));
    }

    #[test]
    fn test_ios_splash() {
        let pwa = MobilePwaConfig::new("App")
            .ios_splash("/splash.png", 1170, 2532);

        let meta = pwa.render_meta_tags();
        assert!(meta.contains("apple-touch-startup-image"));
        assert!(meta.contains("/splash.png"));
    }

    #[test]
    fn test_ios_status_bar() {
        let pwa = MobilePwaConfig::new("App")
            .ios_status_bar(IosStatusBar::BlackTranslucent);

        let meta = pwa.render_meta_tags();
        assert!(meta.contains("black-translucent"));
    }

    #[test]
    fn test_manifest_generation() {
        let pwa = MobilePwaConfig::new("My App")
            .short_name("App")
            .orientation(ScreenOrientation::Portrait)
            .category("productivity");

        let manifest = pwa.render_manifest();
        assert!(manifest.contains("\"My App\""));
        assert!(manifest.contains("\"portrait\""));
        assert!(manifest.contains("\"standalone\""));
        assert!(manifest.contains("productivity"));
    }

    #[test]
    fn test_shortcuts() {
        let pwa = MobilePwaConfig::new("App")
            .shortcut("New Post", "/new")
            .shortcut_with_icon("Search", "/search", "/icons/search.png");

        let manifest = pwa.render_manifest();
        assert!(manifest.contains("New Post"));
        assert!(manifest.contains("/new"));
        assert!(manifest.contains("Search"));
        assert!(manifest.contains("/icons/search.png"));
    }

    #[test]
    fn test_share_target() {
        let pwa = MobilePwaConfig::new("App")
            .share_target("/share", &["title", "text", "url"]);

        let manifest = pwa.render_manifest();
        assert!(manifest.contains("share_target"));
        assert!(manifest.contains("/share"));
        assert!(manifest.contains("\"title\""));
    }

    #[test]
    fn test_share_target_files() {
        let pwa = MobilePwaConfig::new("App")
            .share_target_files("/upload", "images", &["image/png", "image/jpeg"]);

        let manifest = pwa.render_manifest();
        assert!(manifest.contains("images"));
        assert!(manifest.contains("image/png"));
    }

    #[test]
    fn test_push_script() {
        let pwa = MobilePwaConfig::new("App")
            .push_notifications("/api/push/subscribe")
            .vapid_key("BHxVBp...");

        let script = pwa.render_push_script();
        assert!(script.contains("PushManager"));
        assert!(script.contains("/api/push/subscribe"));
        assert!(script.contains("BHxVBp"));
    }

    #[test]
    fn test_no_push_without_config() {
        let pwa = MobilePwaConfig::new("App");
        assert!(pwa.render_push_script().is_empty());
    }

    #[test]
    fn test_background_sync() {
        let pwa = MobilePwaConfig::new("App").background_sync();
        let script = pwa.render_background_sync_script();
        assert!(script.contains("SyncManager"));
        assert!(script.contains("__hayabusa_sync"));
    }

    #[test]
    fn test_install_prompt() {
        let pwa = MobilePwaConfig::new("App");
        let script = pwa.render_install_prompt_script();
        assert!(script.contains("beforeinstallprompt"));
        assert!(script.contains("__hayabusaInstall"));
        assert!(script.contains("hayabusa:installable"));
    }

    #[test]
    fn test_badging() {
        let pwa = MobilePwaConfig::new("App").badging();
        let script = pwa.render_badging_script();
        assert!(script.contains("setAppBadge"));
        assert!(script.contains("clearAppBadge"));
    }

    #[test]
    fn test_no_badging_without_config() {
        let pwa = MobilePwaConfig::new("App");
        assert!(pwa.render_badging_script().is_empty());
    }

    #[test]
    fn test_touch_swipe_back() {
        let gestures = TouchGestureConfig::new()
            .enable_swipe_back()
            .threshold(100);

        let script = gestures.render_script();
        assert!(script.contains("touchstart"));
        assert!(script.contains("touchend"));
        assert!(script.contains("history.back()"));
        assert!(script.contains("threshold=100"));
    }

    #[test]
    fn test_pull_to_refresh() {
        let gestures = TouchGestureConfig::new()
            .enable_pull_to_refresh(Some("/api/refresh"));

        let script = gestures.render_script();
        assert!(script.contains("pullStart"));
        assert!(script.contains("/api/refresh"));
    }

    #[test]
    fn test_pull_to_refresh_default() {
        let gestures = TouchGestureConfig::new()
            .enable_pull_to_refresh(None);

        let script = gestures.render_script();
        assert!(script.contains("location.reload()"));
    }

    #[test]
    fn test_safe_area_css() {
        let css = safe_area_css();
        assert!(css.contains("safe-area-inset-top"));
        assert!(css.contains("safe-area-inset-bottom"));
    }

    #[test]
    fn test_touch_target_css() {
        let css = touch_target_css();
        assert!(css.contains("min-height:44px"));
        assert!(css.contains("font-size:16px"));
        assert!(css.contains("tap-highlight-color"));
    }

    #[test]
    fn test_sw_additions() {
        let sw = mobile_sw_additions();
        assert!(sw.contains("push"));
        assert!(sw.contains("showNotification"));
        assert!(sw.contains("notificationclick"));
        assert!(sw.contains("sync"));
    }

    #[test]
    fn test_related_apps() {
        let pwa = MobilePwaConfig::new("App")
            .related_app("play", "https://play.google.com/store/apps/details?id=com.example");

        let manifest = pwa.render_manifest();
        assert!(manifest.contains("related_applications"));
        assert!(manifest.contains("play"));
    }

    #[test]
    fn test_all_scripts() {
        let pwa = MobilePwaConfig::new("App")
            .push_notifications("/api/push")
            .background_sync()
            .badging();

        let all = pwa.render_all_scripts();
        assert!(all.contains("beforeinstallprompt"));
        assert!(all.contains("PushManager"));
        assert!(all.contains("SyncManager"));
        assert!(all.contains("setAppBadge"));
    }
}
