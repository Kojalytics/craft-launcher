//! The Koja App Suite: the thirteen apps the launcher knows about.
//!
//! The list is data so a later version can fetch it from an API; [`SUITE`] is the built-in copy.
//!
//! Text here is English, the source the UI translates from: each `tagline` has a catalog row
//! keyed by the app id (`tr_ctx(app.id, app.tagline)` in `ui-egui`).

use serde::{Deserialize, Serialize};

/// Which generation of the suite an app belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AppGroup {
    /// The seven original apps.
    Classic,
    /// The six apps added after them.
    New,
}

impl AppGroup {
    pub fn label(self) -> &'static str {
        match self {
            Self::Classic => "Classic",
            Self::New => "New",
        }
    }
}

/// One app in the suite.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CraftApp {
    /// Machine name: repo, binary, package and asset stem (`photocraft`).
    pub id: &'static str,
    /// Display name (`PhotoKoja`).
    pub name: &'static str,
    /// What it's for, in a few words (English; the UI translates it by app id).
    pub tagline: &'static str,
    pub group: AppGroup,
    /// Brand colour (sRGB), for surfaces: kojalytics.com's `--app`.
    pub color: [u8; 3],
    /// The brand colour for text on a light background (the site's light `--app-ink`).
    pub ink_light: [u8; 3],
    /// The brand colour for text on a dark background (the site's dark `--app-ink`).
    pub ink_dark: [u8; 3],
    /// GitHub `owner/repo`.
    pub repo: &'static str,
    /// Older names its release assets may carry (PdfKoja shipped as PrintCraft until 0.2.1).
    pub former_ids: &'static [&'static str],
    /// Older display names its installers registered (PdfKoja's MSI said PrintCraft until 0.2.1).
    pub former_names: &'static [&'static str],
}

impl CraftApp {
    /// `com.kojalytics.<id>`: macOS bundle id, Flatpak id and Linux desktop-file id.
    pub fn bundle_id(&self) -> String {
        format!("com.kojalytics.{}", self.id)
    }

    pub fn releases_url(&self) -> String {
        format!("https://github.com/{}/releases", self.repo)
    }

    pub fn repo_url(&self) -> String {
        format!("https://github.com/{}", self.repo)
    }

    /// The app's page on kojalytics.com.
    pub fn website(&self) -> String {
        "https://www.kojalytics.com/koja-apps".to_string()
    }

    /// Asset stems this app's releases may use, current first.
    pub fn stems(&self) -> impl Iterator<Item = &'static str> + '_ {
        std::iter::once(self.id).chain(self.former_ids.iter().copied())
    }

    /// Display names this app's installers may have registered, current first.
    pub fn names(&self) -> impl Iterator<Item = &'static str> + '_ {
        std::iter::once(self.name).chain(self.former_names.iter().copied())
    }

    pub fn by_id(id: &str) -> Option<&'static CraftApp> {
        SUITE.iter().find(|a| a.id == id)
    }
}

const fn app(
    id: &'static str,
    name: &'static str,
    tagline: &'static str,
    group: AppGroup,
    [color, ink_light, ink_dark]: [[u8; 3]; 3],
    repo: &'static str,
) -> CraftApp {
    CraftApp { id, name, tagline, group, color, ink_light, ink_dark, repo, former_ids: &[], former_names: &[] }
}

/// The apps the launcher shows: those with a released Koja version. Add an app's id here once its
/// Kojalytics release is published; the others stay hidden until then.
pub const LISTED: &[&str] = &[
    "wordcraft", "gridcraft", "deckcraft", "pdfcraft", "photocraft", "lightcraft", "vectorcraft",
    "designcraft", "figcraft", "filmcraft", "effectcraft", "soundcraft", "cadcraft",
];

/// The Koja App Suite, in display order: the seven classic apps, then the six new ones.
pub const SUITE: [CraftApp; 13] = [
    app(
        "photocraft",
        "PhotoKoja",
        "Image editing",
        AppGroup::Classic,
        [[0x2f, 0x7b, 0xf5], [0x1a, 0x5b, 0xc4], [0x7f, 0xb0, 0xff]],
        "Kojalytics/photocraft",
    ),
    app(
        "vectorcraft",
        "VectorKoja",
        "Vector illustration",
        AppGroup::Classic,
        [[0xe8, 0x57, 0x3f], [0xb8, 0x3a, 0x24], [0xff, 0x8a, 0x70]],
        "Kojalytics/vectorcraft",
    ),
    app(
        "filmcraft",
        "FilmKoja",
        "Video editing, color and sound",
        AppGroup::Classic,
        [[0x8b, 0x5c, 0xf6], [0x6a, 0x3f, 0xd6], [0xb6, 0x9c, 0xff]],
        "Kojalytics/filmcraft",
    ),
    app(
        "lightcraft",
        "LightKoja",
        "Photo library and raw development",
        AppGroup::Classic,
        [[0xf2, 0xa5, 0x16], [0x8a, 0x58, 0x00], [0xff, 0xc1, 0x4d]],
        "Kojalytics/lightcraft",
    ),
    CraftApp {
        former_ids: &["printcraft"],
        former_names: &["PrintCraft"],
        ..app(
            "pdfcraft",
            "PdfKoja",
            "Reading, organizing and protecting PDFs",
            AppGroup::Classic,
            [[0x12, 0xa5, 0x8a], [0x0a, 0x75, 0x63], [0x3f, 0xd6, 0xb8]],
            "Kojalytics/pdfcraft",
        )
    },
    app(
        "effectcraft",
        "EffectKoja",
        "Motion graphics and visual effects",
        AppGroup::Classic,
        [[0xe0, 0x36, 0x8f], [0xb0, 0x20, 0x6c], [0xff, 0x7a, 0xb8]],
        "Kojalytics/effectcraft",
    ),
    app(
        "designcraft",
        "DesignKoja",
        "Page layout and publishing",
        AppGroup::Classic,
        [[0x7b, 0xb5, 0x1c], [0x45, 0x70, 0x08], [0xa6, 0xe0, 0x4a]],
        "Kojalytics/designcraft",
    ),
    app(
        "wordcraft",
        "WordKoja",
        "Writing and document design",
        AppGroup::New,
        [[0x3f, 0x5f, 0xe0], [0x2a, 0x44, 0xb8], [0x8e, 0xa2, 0xff]],
        "Kojalytics/wordcraft",
    ),
    app(
        "deckcraft",
        "DeckKoja",
        "Presentations and slide shows",
        AppGroup::New,
        [[0xf0, 0x7a, 0x1e], [0xa8, 0x4f, 0x0a], [0xff, 0xa0, 0x60]],
        "Kojalytics/deckcraft",
    ),
    app(
        "gridcraft",
        "GridKoja",
        "Spreadsheets and data",
        AppGroup::New,
        [[0x1f, 0x9d, 0x55], [0x12, 0x70, 0x3a], [0x4f, 0xd6, 0x8a]],
        "Kojalytics/gridcraft",
    ),
    app(
        "cadcraft",
        "CADKoja",
        "Computer-aided design and drafting",
        AppGroup::New,
        [[0x3e, 0xa6, 0xe8], [0x1a, 0x6f, 0xa8], [0x7c, 0xc8, 0xff]],
        "Kojalytics/cadcraft",
    ),
    app(
        "soundcraft",
        "SoundKoja",
        "Recording, editing and mixing audio",
        AppGroup::New,
        [[0x1f, 0xa5, 0xc9], [0x0e, 0x72, 0x90], [0x5f, 0xd0, 0xee]],
        "Kojalytics/soundcraft",
    ),
    app(
        "figcraft",
        "FigKoja",
        "Interface design and prototyping",
        AppGroup::New,
        [[0x02, 0xda, 0x96], [0x04, 0x7a, 0x56], [0x4f, 0xe8, 0xb6]],
        "Kojalytics/figcraft",
    ),
];

/// Kojalytics's website.
pub const WEBSITE_URL: &str = "https://kojalytics.com";
/// The GitHub account every Craft App's repository lives under: the only one the launcher
/// installs from, whatever the release feed says.
pub const GITHUB_OWNER: &str = "Kojalytics";
/// Kojalytics on GitHub: every Craft App's source.
pub const GITHUB_URL: &str = "https://github.com/Kojalytics";
/// Kojalytics's social accounts: (service, URL), shown as small icons. Add them here.
pub const SOCIAL: [(&str, &str); 0] = [];

/// Kojalytics Launcher itself, described like a Craft App so it can update itself with the same
/// machinery (asset names `artcraft-launcher-<ver>-…`, bundle id `com.kojalytics.artcraft-launcher`,
/// `Kojalytics Launcher.app`). Not part of [`SUITE`].
pub const LAUNCHER: CraftApp = app(
    "artcraft-launcher",
    "Kojalytics Launcher",
    "Installs and updates the Koja App Suite",
    AppGroup::New,
    [[0x0b, 0x0b, 0x0c], [0x0b, 0x0b, 0x0c], [0xf2, 0xf1, 0xee]],
    "Kojalytics/craft-launcher",
);

/// The launcher's own repo, checked so it can say when a newer launcher is out.
pub const LAUNCHER_REPO: &str = "Kojalytics/craft-launcher";
/// The launcher's own asset stem.
pub const LAUNCHER_ID: &str = "artcraft-launcher";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suite_has_seven_classic_and_six_new_apps_with_unique_ids() {
        assert_eq!(SUITE.iter().filter(|a| a.group == AppGroup::Classic).count(), 7);
        assert_eq!(SUITE.iter().filter(|a| a.group == AppGroup::New).count(), 6);
        let mut ids: Vec<_> = SUITE.iter().map(|a| a.id).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), SUITE.len());
        for a in &SUITE {
            assert_eq!(a.repo, format!("Kojalytics/{}", a.id));
            assert_eq!(a.name.to_lowercase().replace("koja", "craft"), a.id);
        }
    }

    #[test]
    fn pdfcraft_also_answers_to_printcraft() {
        let pdf = CraftApp::by_id("pdfcraft").unwrap();
        assert_eq!(pdf.stems().collect::<Vec<_>>(), ["pdfcraft", "printcraft"]);
        assert_eq!(pdf.names().collect::<Vec<_>>(), ["PdfKoja", "PrintCraft"]);
        assert_eq!(pdf.website(), "https://www.kojalytics.com/koja-apps");
        assert_eq!(CraftApp::by_id("wordcraft").unwrap().website(), "https://www.kojalytics.com/koja-apps");
    }
}
