//! 持ち主が PC Status に出したくない項目 (`PCSC_HIDE=gpu_usage,temperature`)。
//! 隠した項目は送らず（または 0 で送り）、名前を status の `hidden` に載せます。

pub const ITEMS: [(&str, &str); 15] = [
    ("os", "OS の名前とバージョン"),
    ("cpu", "コアごとの CPU 使用率"),
    ("cpu_name", "CPU 名とコアの種類"),
    ("cpu_frequency", "コアの最高周波数"),
    ("memory", "メモリとスワップ"),
    ("storage", "ディスク"),
    ("gpu", "GPU すべて"),
    ("gpu_name", "GPU 名"),
    ("gpu_usage", "GPU 使用率"),
    ("gpu_memory", "GPU メモリ"),
    ("temperature", "温度センサー（GPU も）"),
    ("battery", "バッテリー"),
    ("uptime", "起動からの時間"),
    ("loadavg", "ロードアベレージ"),
    ("iowait", "IO wait"),
];

const GPU_PARTS: [&str; 3] = ["gpu_name", "gpu_usage", "gpu_memory"];

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Hidden(Vec<&'static str>);

impl Hidden {
    /// カンマか空白区切り、大文字小文字は問いません。知らない名前も返します。
    pub fn parse(value: &str) -> (Self, Vec<String>) {
        let mut items = Vec::new();
        let mut unknown = Vec::new();
        for name in value
            .split(|c: char| c == ',' || c.is_whitespace())
            .map(str::trim)
            .filter(|n| !n.is_empty())
        {
            let name = name.to_ascii_lowercase();
            match ITEMS.iter().find(|(item, _)| *item == name) {
                Some((item, _)) if !items.contains(item) => items.push(*item),
                Some(_) => {}
                None => unknown.push(name),
            }
        }
        if items.contains(&"gpu") {
            items.retain(|item| !GPU_PARTS.contains(item));
        }
        items.sort_unstable();
        (Self(items), unknown)
    }

    pub fn has(&self, item: &str) -> bool {
        self.0.contains(&item) || (GPU_PARTS.contains(&item) && self.0.contains(&"gpu"))
    }

    pub fn names(&self) -> &[&'static str] {
        &self.0
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// 取れない項目を自動で隠す（利用者の指定とは別に足します）。
    pub fn with(mut self, item: &'static str) -> Self {
        if !self.has(item) {
            self.0.push(item);
            self.0.sort_unstable();
        }
        self
    }
}
