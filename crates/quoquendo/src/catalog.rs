//! The voice installers of the Loquendo 7 pack on archive.org
//! (`text-to-speech-oddcast-voices-pack-2024`, folder `Loquendo 7`): file,
//! size, and the voice each installs (Raffaele installs Roberto).

/// One installer of the pack.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Installer {
    /// its file name (`Raffaele - Italian.exe`)
    pub file: &'static str,
    /// the name on the file
    pub label: &'static str,
    /// the language on the file
    pub language: &'static str,
    /// the `BaseVoice` it installs
    pub voice: &'static str,
    pub gender: &'static str,
    /// bytes
    pub size: u64,
}

pub const INSTALLERS: [Installer; 47] = [
    Installer { file: "Afroditi - Greek.exe", label: "Afroditi", language: "Greek", voice: "Afroditi", gender: "Female", size: 50011977 },
    Installer { file: "Alan - Australian English.exe", label: "Alan", language: "Australian English", voice: "Alan", gender: "Male", size: 61492224 },
    Installer { file: "Allison - US English.exe", label: "Allison", language: "US English", voice: "Allison", gender: "Female", size: 57322123 },
    Installer { file: "Amalia - European Portuguese.exe", label: "Amalia", language: "European Portuguese", voice: "Amalia", gender: "Female", size: 67905434 },
    Installer { file: "Annika - Swedish.exe", label: "Annika", language: "Swedish", voice: "Annika", gender: "Female", size: 46939401 },
    Installer { file: "Bernard - French.exe", label: "Bernard", language: "French", voice: "Bernard", gender: "Male", size: 51293953 },
    Installer { file: "Carlos - Colombia Spanish.exe", label: "Carlos", language: "Colombia Spanish", voice: "Carlos", gender: "Male", size: 57594192 },
    Installer { file: "Carmela - Galician.exe", label: "Carmela", language: "Galician", voice: "Carmela", gender: "Female", size: 45793280 },
    Installer { file: "Carmen - European Spanish.exe", label: "Carmen", language: "European Spanish", voice: "Carmen", gender: "Female", size: 52051369 },
    Installer { file: "Catherine - UK English.exe", label: "Catherine", language: "UK English", voice: "Kate", gender: "Female", size: 70328215 },
    Installer { file: "Dave - US English.exe", label: "Dave", language: "US English", voice: "Dave", gender: "Male", size: 65374438 },
    Installer { file: "Diego - Argentine Spanish.exe", label: "Diego", language: "Argentine Spanish", voice: "Diego", gender: "Male", size: 45120562 },
    Installer { file: "Dmitri - Russian.exe", label: "Dmitri", language: "Russian", voice: "Dmitri", gender: "Male", size: 68036163 },
    Installer { file: "Elizabeth - UK English.exe", label: "Elizabeth", language: "UK English", voice: "Elizabeth", gender: "Female", size: 55569797 },
    Installer { file: "Empar - Valencian Catalan.exe", label: "Empar", language: "Valencian Catalan", voice: "Empar", gender: "Female", size: 67425280 },
    Installer { file: "Esperanza - Mexican Spanish.exe", label: "Esperanza", language: "Mexican Spanish", voice: "Esperanza", gender: "Female", size: 50719785 },
    Installer { file: "Eusebio - European Portuguese.exe", label: "Eusebio", language: "European Portuguese", voice: "Eusebio", gender: "Male", size: 53562368 },
    Installer { file: "Felipe - Brazilian Portuguese.EXE", label: "Felipe", language: "Brazilian Portuguese", voice: "Felipe", gender: "Male", size: 52724736 },
    Installer { file: "Fernanda - Brazilian Portuguese.exe", label: "Fernanda", language: "Brazilian Portuguese", voice: "Fernanda", gender: "Female", size: 68588354 },
    Installer { file: "Florence - French.exe", label: "Florence", language: "French", voice: "Florence", gender: "Female", size: 54070272 },
    Installer { file: "Francisca - Chilean Spanish.exe", label: "Francisca", language: "Chilean Spanish", voice: "Francisca", gender: "Female", size: 46621363 },
    Installer { file: "Frida - Danish.exe", label: "Frida", language: "Danish", voice: "Frida", gender: "Female", size: 55224689 },
    Installer { file: "Gabriela - Brazilian Portuguese.exe", label: "Gabriela", language: "Brazilian Portuguese", voice: "Gabriela", gender: "Female", size: 63031068 },
    Installer { file: "Giulia - Italian.exe", label: "Giulia", language: "Italian", voice: "Giulia", gender: "Female", size: 59893348 },
    Installer { file: "Grace - Australian English.exe", label: "Grace", language: "Australian English", voice: "Grace", gender: "Female", size: 68022784 },
    Installer { file: "Jolie - French.exe", label: "Jolie", language: "French", voice: "Juliette", gender: "Female", size: 55537481 },
    Installer { file: "Jordi - Catalan.exe", label: "Jordi", language: "Catalan", voice: "Jordi", gender: "Male", size: 51994624 },
    Installer { file: "Jorge - European Spanish.exe", label: "Jorge", language: "European Spanish", voice: "Jorge", gender: "Male", size: 47893037 },
    Installer { file: "Katrin - German.exe", label: "Katrin", language: "German", voice: "Katrin", gender: "Female", size: 69003336 },
    Installer { file: "Krzysztof - Polish.exe", label: "Krzysztof", language: "Polish", voice: "Krzysztof", gender: "Male", size: 57703198 },
    Installer { file: "Leonor - European Spanish.exe", label: "Leonor", language: "European Spanish", voice: "Leonor", gender: "Female", size: 43055104 },
    Installer { file: "LinLin - Chinese.exe", label: "LinLin", language: "Chinese", voice: "Linlin", gender: "Female", size: 32172544 },
    Installer { file: "Luca - Italian.exe", label: "Luca", language: "Italian", voice: "Luca", gender: "Male", size: 57878997 },
    Installer { file: "Marcello - Italian.exe", label: "Marcello", language: "Italian", voice: "Marcello", gender: "Male", size: 51138048 },
    Installer { file: "Matteo - Italian.exe", label: "Matteo", language: "Italian", voice: "Matteo", gender: "Male", size: 57640492 },
    Installer { file: "Olga - Russian.exe", label: "Olga", language: "Russian", voice: "Olga", gender: "Female", size: 66356200 },
    Installer { file: "Paola - Italian.exe", label: "Paola", language: "Italian", voice: "Paola", gender: "Female", size: 60046486 },
    Installer { file: "Raffaele - Italian.exe", label: "Raffaele", language: "Italian", voice: "Roberto", gender: "Male", size: 52762973 },
    Installer { file: "Simon - UK English.exe", label: "Simon", language: "UK English", voice: "Simon", gender: "Male", size: 66891693 },
    Installer { file: "Soledad - Colombia Spanish.exe", label: "Soledad", language: "Colombia Spanish", voice: "Soledad", gender: "Female", size: 52802628 },
    Installer { file: "Stefan - German.exe", label: "Stefan", language: "German", voice: "Stefan", gender: "Male", size: 65896124 },
    Installer { file: "Steven - US English.exe", label: "Steven", language: "US English", voice: "Steven", gender: "Male", size: 55203131 },
    Installer { file: "Susan - US English.exe", label: "Susan", language: "US English", voice: "Susan", gender: "Female", size: 68184348 },
    Installer { file: "Sven - Swedish.exe", label: "Sven", language: "Swedish", voice: "Sven", gender: "Male", size: 70584218 },
    Installer { file: "Willem - Dutch.exe", label: "Willem", language: "Dutch", voice: "Willem", gender: "Male", size: 84391151 },
    Installer { file: "Ximena - Colombia Spanish.exe", label: "Ximena", language: "Colombia Spanish", voice: "Ximena", gender: "Female", size: 55499264 },
    Installer { file: "Zosia - Polish.exe", label: "Zosia", language: "Polish", voice: "Zosia", gender: "Female", size: 61304127 },
];

/// MotherTongues whose language module ships in no installer of the pack
/// (Jordi, Empar, Carmela, LinLin): their voices can't be read properly yet.
pub const NO_MODULE: [&str; 4] = ["Catalan", "Valencian", "Galician", "Chinese"];

/// Where archive.org serves an installer (with ranges; its storage nodes
/// send no CORS header for `.exe` files, so browsers can't fetch it directly).
pub fn archive_url(file: &str) -> String {
    let mut url = String::from("https://archive.org/download/text-to-speech-oddcast-voices-pack-2024/Loquendo%207/");
    for b in file.bytes() {
        if b.is_ascii_alphanumeric() || b"-._~".contains(&b) {
            url.push(b as char);
        } else {
            url.push_str(&format!("%{b:02X}"));
        }
    }
    url
}

/// The installer that brings `voice` (`BaseVoice`, any case).
pub fn installer_for(voice: &str) -> Option<&'static Installer> {
    INSTALLERS.iter().find(|i| i.voice.eq_ignore_ascii_case(voice))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn urls_are_percent_encoded() {
        assert_eq!(
            archive_url("Felipe - Brazilian Portuguese.EXE"),
            "https://archive.org/download/text-to-speech-oddcast-voices-pack-2024/Loquendo%207/Felipe%20-%20Brazilian%20Portuguese.EXE"
        );
        assert_eq!(installer_for("roberto").unwrap().file, "Raffaele - Italian.exe");
    }
}
