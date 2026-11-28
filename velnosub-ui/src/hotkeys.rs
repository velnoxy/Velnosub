use eframe::egui::{Key, KeyboardShortcut, Modifiers};
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub(crate) enum Action {
	Palette, Save, SaveAs, Open, New, Find, ShiftTimes, Redo, Undo, SelectAll, Duplicate, Delete,
}

const SHIFT_COMMAND: Modifiers = Modifiers { shift: true, ..Modifiers::COMMAND };
pub(crate) const ACTIONS: &[(Action, &str, &str, &[(Modifiers, Key)])] = &[
	(Action::Palette, "palette", "Komut paleti", &[(Modifiers::COMMAND, Key::K)]),
	(Action::SaveAs, "save-as", "Farklı kaydet", &[(SHIFT_COMMAND, Key::S)]),
	(Action::Save, "save", "Kaydet", &[(Modifiers::COMMAND, Key::S), (Modifiers::NONE, Key::F2)]),
	(Action::Open, "open", "Altyazı aç", &[(Modifiers::COMMAND, Key::O)]),
	(Action::New, "new", "Yeni altyazı", &[(Modifiers::COMMAND, Key::N)]),
	(Action::Find, "find", "Ara ve değiştir", &[(Modifiers::COMMAND, Key::H)]),
	(Action::ShiftTimes, "shift", "Zaman kaydır", &[(Modifiers::COMMAND, Key::I)]),
	(Action::Redo, "redo", "İleri al", &[(SHIFT_COMMAND, Key::Z), (Modifiers::COMMAND, Key::Y)]),
	(Action::Undo, "undo", "Geri al", &[(Modifiers::COMMAND, Key::Z)]),
	(Action::SelectAll, "select-all", "Tüm satırları seç", &[(Modifiers::COMMAND, Key::A)]),
	(Action::Duplicate, "duplicate", "Satırları çoğalt", &[(Modifiers::COMMAND, Key::D)]),
	(Action::Delete, "delete", "Satırları sil", &[(Modifiers::NONE, Key::Delete)]),
];

#[derive(Default)]
pub(crate) struct Hotkeys {
	custom: BTreeMap<Action, KeyboardShortcut>,
}

pub(crate) fn normal_modifiers(modifiers: Modifiers) -> Modifiers {
	Modifiers {
		shift: modifiers.shift,
		alt: modifiers.alt,
		command: modifiers.command || modifiers.ctrl || modifiers.mac_cmd,
		..Modifiers::NONE
	}
}

impl Hotkeys {
	pub fn load(path: &Path) -> Self {
		let mut settings = Self::default();
		if let Ok(content) = std::fs::read_to_string(path) {
			for line in content.lines() {
				let Some((id, value)) = line.split_once('=') else { continue };
				let Some(&(action, _, _, _)) = ACTIONS.iter().find(|(_, name, _, _)| *name == id) else { continue };
				let Some((mods, key)) = value.split_once(':') else { continue };
				let Some(key) = Key::from_name(key) else { continue };
				let Ok(mods) = mods.parse::<u8>() else { continue };
				if mods > 7 { continue; }
				let modifiers = Modifiers {
					shift: mods & 1 != 0,
					alt: mods & 2 != 0,
					command: mods & 4 != 0,
					..Modifiers::NONE
				};
				let _ = settings.set(action, KeyboardShortcut::new(modifiers, key));
			}
		}
		settings
	}

	pub fn save(&self, path: &Path) -> std::io::Result<()> {
		use std::fmt::Write;
		let mut content = String::new();
		for (action, id, _, _) in ACTIONS {
			if let Some(shortcut) = self.custom.get(action) {
				let m = shortcut.modifiers;
				let bits = u8::from(m.shift) + 2 * u8::from(m.alt) + 4 * u8::from(m.command);
				writeln!(&mut content, "{id}={bits}:{}", shortcut.logical_key.name()).unwrap();
			}
		}
		let temporary = path.with_extension("keys-tmp");
		std::fs::write(&temporary, content)?;
		std::fs::rename(temporary, path)
	}

	pub fn shortcuts(&self, action: Action) -> Vec<KeyboardShortcut> {
		if let Some(&shortcut) = self.custom.get(&action) { return vec![shortcut]; }
		ACTIONS.iter().find(|(candidate, ..)| *candidate == action).unwrap().3.iter()
			.map(|&(modifiers, key)| KeyboardShortcut::new(modifiers, key)).collect()
	}

	pub fn consume(&self, ui: &eframe::egui::Ui, action: Action) -> bool {
		ui.input_mut(|input| {
			let index = input.events.iter().position(|event| match event {
				eframe::egui::Event::Key { key, modifiers, pressed: true, repeat: false, .. } => {
					let modifiers = normal_modifiers(*modifiers);
					if let Some(shortcut) = self.custom.get(&action) {
						shortcut.logical_key == *key && shortcut.modifiers == modifiers
					} else {
						ACTIONS.iter().find(|(candidate, ..)| *candidate == action).unwrap().3
							.contains(&(modifiers, *key))
					}
				}
				_ => false,
			});
			if let Some(index) = index { input.events.remove(index); }
			index.is_some()
		})
	}

	pub fn set(&mut self, action: Action, shortcut: KeyboardShortcut) -> Result<(), &'static str> {
		for &(other, _, label, _) in ACTIONS {
			if other != action && self.shortcuts(other).contains(&shortcut) {
				return Err(label);
			}
		}
		self.custom.insert(action, shortcut);
		Ok(())
	}

	pub fn reset(&mut self) { self.custom.clear(); }
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn custom_shortcut_survives_reload_and_rejects_conflicts() {
		let path = std::env::temp_dir().join(format!("velnosub-hotkey-test-{}.keys", std::process::id()));
		let mut settings = Hotkeys::default();
		let existing = KeyboardShortcut::new(Modifiers::COMMAND, Key::S);
		assert!(settings.set(Action::Open, existing).is_err());
		let replacement = KeyboardShortcut::new(Modifiers::ALT, Key::F3);
		settings.set(Action::Open, replacement).unwrap();
		settings.save(&path).unwrap();
		let mut restored = Hotkeys::load(&path);
		assert_eq!(restored.shortcuts(Action::Open), [replacement]);
		restored.reset();
		assert_eq!(restored.shortcuts(Action::Open), [KeyboardShortcut::new(Modifiers::COMMAND, Key::O)]);
		std::fs::remove_file(path).unwrap();
	}
}
