//! Tool definitions

pub struct ToolDef {
    pub name: &'static str,
    pub repo: &'static str,
    pub asset_pattern: &'static [&'static str],
}

#[cfg(target_os = "linux")]
pub static TOOLS: &[ToolDef] = &[
    ToolDef {
        name: "AssetRipper",
        repo: "AssetRipper/AssetRipper",
        asset_pattern: &["linux", "Linux"],
    },
    ToolDef {
        name: "Il2CppDumper",
        repo: "Perfare/Il2CppDumper",
        asset_pattern: &["net6"],
    },
];

#[cfg(target_os = "macos")]
pub static TOOLS: &[ToolDef] = &[
    ToolDef {
        name: "AssetRipper",
        repo: "AssetRipper/AssetRipper",
        asset_pattern: &["mac", "osx", "darwin"],
    },
    ToolDef {
        name: "Il2CppDumper",
        repo: "Perfare/Il2CppDumper",
        asset_pattern: &["net6"],
    },
];

#[cfg(target_os = "windows")]
pub static TOOLS: &[ToolDef] = &[
    ToolDef {
        name: "AssetRipper",
        repo: "AssetRipper/AssetRipper",
        asset_pattern: &["win", "windows"],
    },
    ToolDef {
        name: "Il2CppDumper",
        repo: "Perfare/Il2CppDumper",
        asset_pattern: &["net6", "win"],
    },
];
