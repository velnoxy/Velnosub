//! ASS override etiketi oku/yaz/sil — egui/ABI bağımsız saf yardımcılar.
//!
//! İşlemler ilk `{...}` override bloğunu hedefler; blok yoksa `etiket_yaz`
//! başa yeni blok açar. Diğer bloklar ve satır metni aynen korunur. `\t(...)`
//! dönüşümünün içindeki etiketler satır düzeyi sayılmaz.

#[derive(Clone, Debug)]
struct Etiket {
	ad: String,
	bas: usize,
	son: usize,
	deger_bas: usize,
	deger_son: usize,
}

/// İlk `{...}` bloğunun (içerik başı, içerik sonu, blok sonu) bayt aralığı.
fn ilk_blok(metin: &str) -> Option<(usize, usize, usize)> {
	let b = metin.find('{')?;
	let ic = b + 1;
	let k = metin[ic..].find('}')?;
	let c = ic + k;
	Some((ic, c, c + 1))
}

fn ad_temizle(ad: &str) -> &str {
	ad.trim().trim_start_matches('\\')
}

/// Human-readable portions of an ASS event. Keep overrides, comments,
/// drawing commands and ASS line-break/space controls outside these ranges.
pub(crate) fn translation_ranges(text: &str) -> Vec<std::ops::Range<usize>> {
	let mut ranges = Vec::new();
	let mut cursor = 0;
	let mut drawing = false;
	while cursor < text.len() {
		if text[cursor..].starts_with('{') {
			if let Some(close) = text[cursor..].find('}') {
				let end = cursor + close;
				for tag in etiketler(&text[cursor + 1..end]) {
					if tag.ad == "p" {
						let value = &text[cursor + 1 + tag.deger_bas..cursor + 1 + tag.deger_son];
						drawing = value.trim().parse::<i32>().unwrap_or(0) != 0;
					}
				}
				cursor = end + 1;
				continue;
			}
		}
		let after_first_char = cursor + text[cursor..].chars().next().unwrap().len_utf8();
		let end = text[after_first_char..].find('{').map_or(text.len(), |i| after_first_char + i);
		if !drawing {
			let mut start = cursor;
			let mut chars = text[cursor..end].char_indices().peekable();
			while let Some((offset, ch)) = chars.next() {
				if ch == '\\' && chars.peek().is_some_and(|(_, ch)| matches!(ch, 'N' | 'n' | 'h')) {
					let control = cursor + offset;
					if !text[start..control].trim().is_empty() {
						ranges.push(start..control);
					}
					chars.next();
					start = control + 2;
				}
			}
			if !text[start..end].trim().is_empty() {
				ranges.push(start..end);
			}
		}
		cursor = end;
	}
	ranges
}

#[cfg(test)]
#[test]
fn translation_keeps_ass_structure() {
	let source = r"{\pos(100,200)\k30}İlk{\k25} ikinci\N{\p1}m 0 0 l 8 8{\p0}{not}最後\h!";
	let ranges = translation_ranges(source);
	let originals: Vec<_> = ranges.iter().map(|range| &source[range.clone()]).collect();
	assert_eq!(originals, ["İlk", " ikinci", "最後", "!"]);
	let mut translated = source.to_owned();
	for (range, value) in ranges.into_iter().zip(["First", " second", "Last", "!"]).rev() {
		translated.replace_range(range, value);
	}
	assert_eq!(translated, r"{\pos(100,200)\k30}First{\k25} second\N{\p1}m 0 0 l 8 8{\p0}{not}Last\h!");
	assert_eq!(translation_ranges("Önce {eksik"), [0..6, 6..12]);
}

/// Bir override bloğundaki etiketleri bayt aralıklarıyla çıkar.
fn etiketler(blok: &str) -> Vec<Etiket> {
	let mut out = Vec::new();
	let mut i = 0usize;
	while i < blok.len() {
		let c = blok[i..].chars().next().unwrap();
		if c != '\\' {
			i += c.len_utf8();
			continue;
		}
		let bas = i;
		let mut j = i + 1;
		while j < blok.len() {
			let c = blok[j..].chars().next().unwrap();
			if c.is_ascii_alphabetic() {
				j += c.len_utf8();
			} else {
				break;
			}
		}
		if j == i + 1 {
			i += 1;
			continue;
		}
		let ad = &blok[i + 1..j];
		let mut vb = j;
		while vb < blok.len() {
			let c = blok[vb..].chars().next().unwrap();
			if c.is_whitespace() {
				vb += c.len_utf8();
			} else {
				break;
			}
		}
		let mut vs = vb;
		if blok[vs..].starts_with('(') {
			let mut derinlik = 0i32;
			let mut k = vs;
			while k < blok.len() {
				let c = blok[k..].chars().next().unwrap();
				if c == '(' {
					derinlik += 1;
				} else if c == ')' {
					derinlik -= 1;
					if derinlik == 0 {
						k += c.len_utf8();
						break;
					}
				}
				k += c.len_utf8();
			}
			vs = if derinlik == 0 { k } else { blok.len() };
		} else {
			while vs < blok.len() && !blok[vs..].starts_with('\\') {
				let c = blok[vs..].chars().next().unwrap();
				vs += c.len_utf8();
			}
		}
		if !ad.eq_ignore_ascii_case("t") {
			out.push(Etiket {
				ad: ad.to_ascii_lowercase(),
				bas,
				son: vs,
				deger_bas: vb,
				deger_son: vs,
			});
		}
		i = vs.max(j);
	}
	out
}

/// İlk override bloğundaki `ad` etiketinin değerini döndür (ham, parantezli).
pub fn etiket_al(metin: &str, ad: &str) -> Option<String> {
	let ad = ad_temizle(ad).to_ascii_lowercase();
	let (ic, son, _) = ilk_blok(metin)?;
	etiketler(&metin[ic..son])
		.into_iter()
		.find(|e| e.ad == ad)
		.map(|e| metin[ic + e.deger_bas..ic + e.deger_son].to_string())
}

/// `ad` etiketini ilk blokta güncelle; yoksa aynı bloğun sonuna ekle.
/// Blok yoksa metnin başına yeni blok aç. Diğer metin aynen korunur.
pub fn etiket_yaz(metin: &str, ad: &str, deger: &str) -> String {
	let ad = ad_temizle(ad);
	if ad.is_empty() {
		return metin.to_string();
	}
	let Some((ic, son, _)) = ilk_blok(metin) else {
		return format!("{{\\{ad}{deger}}}{metin}");
	};
	let blok = &metin[ic..son];
	if let Some(e) = etiketler(blok).into_iter().find(|e| e.ad == ad.to_ascii_lowercase()) {
		let mut out = String::with_capacity(metin.len() + deger.len());
		out.push_str(&metin[..ic + e.deger_bas]);
		out.push_str(deger);
		out.push_str(&metin[ic + e.deger_son..]);
		return out;
	}
	let mut out = String::with_capacity(metin.len() + ad.len() + deger.len() + 1);
	out.push_str(&metin[..son]);
	out.push('\\');
	out.push_str(ad);
	out.push_str(deger);
	out.push_str(&metin[son..]);
	out
}

/// `ad` etiketinin ilk blokta görünen tüm örneklerini sil.
pub fn etiket_sil(metin: &str, ad: &str) -> String {
	let ad = ad_temizle(ad).to_ascii_lowercase();
	let Some((ic, son, _)) = ilk_blok(metin) else {
		return metin.to_string();
	};
	let mut out = metin.to_string();
	for e in etiketler(&metin[ic..son])
		.into_iter()
		.filter(|e| e.ad == ad)
		.rev()
	{
		out.replace_range(ic + e.bas..ic + e.son, "");
	}
	out
}

/// Değerdeki ilk sayıyı çöz; `"(10,20)"`, `"10"` ve `"m 0 0"` desteklenir.
pub fn sayi_al(deger: &str) -> Option<f64> {
	for tok in deger.split(|c: char| {
		c == ',' || c == '(' || c == ')' || c.is_whitespace()
	}) {
		if let Ok(v) = tok.parse::<f64>() {
			if v.is_finite() {
				return Some(v);
			}
		}
	}
	None
}

struct ClipPath {
	scale: Option<u8>,
	segments: Vec<(char, Vec<(f64, f64)>)>,
}

impl ClipPath {
	fn complex(&self) -> bool {
		self.scale.unwrap_or(1) != 1
			|| self.segments.iter().skip(1).any(|(command, _)| *command != 'l')
			|| self.segments.first().is_some_and(|(command, _)| *command != 'm')
	}

	fn factor(&self) -> f64 {
		f64::from(1u32 << (self.scale.unwrap_or(1) - 1))
	}
}

fn clip_path(metin: &str, ad: &str) -> Option<ClipPath> {
	let raw = etiket_al(metin, ad)?;
	let drawing = raw.trim().strip_prefix('(')?.strip_suffix(')')?.trim();
	let (scale, drawing) = if let Some((head, tail)) = drawing.split_once(',') {
		let scale: u8 = head.trim().parse().ok()?;
		if !(1..=30).contains(&scale) { return None; }
		(Some(scale), tail.trim())
	} else { (None, drawing) };
	let mut commands: Vec<(char, Vec<f64>)> = Vec::new();
	for token in drawing.split_whitespace() {
		if let Some(command) = token.chars().next().filter(|_| token.len() == 1)
			.filter(|command| matches!(command, 'm' | 'n' | 'l' | 'b' | 's' | 'p' | 'c')) {
			commands.push((command, Vec::new()));
		} else {
			let number: f64 = token.parse().ok()?;
			if !number.is_finite() { return None; }
			commands.last_mut()?.1.push(number);
		}
	}
	if !matches!(commands.first()?.0, 'm' | 'n') { return None; }
	let segments = commands.into_iter().map(|(command, coordinates)| {
		let length = coordinates.len();
		let valid = match command {
			'm' | 'n' => length == 2,
			'l' | 'p' => length >= 2 && length % 2 == 0,
			'b' => length >= 6 && length % 6 == 0,
			's' => length >= 6 && length % 2 == 0,
			'c' => length == 0,
			_ => false,
		};
		valid.then(|| (command, coordinates.chunks_exact(2).map(|xy| (xy[0], xy[1])).collect()))
	}).collect::<Option<_>>()?;
	Some(ClipPath { scale, segments })
}

pub fn vektor_noktalari(metin: &str, ad: &str) -> Option<Vec<(f64, f64)>> {
	let path = clip_path(metin, ad)?;
	let factor = path.factor();
	Some(path.segments.into_iter().flat_map(|(_, points)| points)
		.map(|(x, y)| (x / factor, y / factor)).collect())
}

pub(crate) fn complex_clip(metin: &str, ad: &str) -> bool {
	clip_path(metin, ad).is_some_and(|path| path.complex())
}

pub(crate) fn clip_segments(metin: &str, ad: &str) -> Option<Vec<(char, usize)>> {
	let path = clip_path(metin, ad).filter(ClipPath::complex)?;
	Some(path.segments.into_iter().map(|(command, points)| (command, points.len())).collect())
}

fn sayi_yaz(v: f64) -> String {
	if v.fract() == 0.0 {
		format!("{v:.0}")
	} else {
		let mut s = format!("{v:.3}");
		while s.ends_with('0') {
			s.pop();
		}
		if s.ends_with('.') {
			s.pop();
		}
		s
	}
}

/// Noktaları `\clip(m x y l ...)` biçiminde yaz; `kapali` ise ilk noktaya
/// dönen bir `l` eklenir. Boş nokta listesi etiketi siler.
pub fn vektor_yaz(
	metin: &str,
	ad: &str,
	noktalar: &[(f64, f64)],
	kapali: bool,
) -> String {
	if noktalar.is_empty() {
		return etiket_sil(metin, ad);
	}
	if let Some(path) = clip_path(metin, ad).filter(ClipPath::complex) {
		if path.segments.iter().map(|(_, points)| points.len()).sum::<usize>() != noktalar.len() {
			return metin.to_owned();
		}
		let mut value = match path.scale {
			Some(scale) => format!("({scale},"),
			None => "(".to_owned(),
		};
		let factor = path.factor();
		let mut points = noktalar.iter();
		for (index, (command, old_points)) in path.segments.into_iter().enumerate() {
			if index > 0 { value.push(' '); }
			value.push(command);
			for _ in old_points {
				let &(x, y) = points.next().unwrap();
				value.push_str(&format!(" {} {}", sayi_yaz(x * factor), sayi_yaz(y * factor)));
			}
		}
		value.push(')');
		return etiket_yaz(metin, ad, &value);
	}
	let ilk = noktalar[0];
	let mut deger = format!("(m {} {}", sayi_yaz(ilk.0), sayi_yaz(ilk.1));
	for (x, y) in noktalar.iter().skip(1) {
		deger.push_str(&format!(" l {} {}", sayi_yaz(*x), sayi_yaz(*y)));
	}
	if kapali && noktalar.len() >= 3 {
		deger.push_str(&format!(" l {} {}", sayi_yaz(ilk.0), sayi_yaz(ilk.1)));
	}
	deger.push(')');
	etiket_yaz(metin, ad, &deger)
}

#[cfg(test)]
mod testler {
	use super::*;

	#[test]
	fn scaled_bezier_edit_preserves_control_points_and_path_commands() {
		let source = r"{\clip(3,m 0 0 b 24 32 40 48 64 64 l 0 0)}x";
		assert!(complex_clip(source, "clip"));
		let mut points = vektor_noktalari(source, "clip").unwrap();
		assert_eq!(points[1], (6.0, 8.0));
		points[1] = (7.0, 8.0);
		let edited = vektor_yaz(source, "clip", &points, true);
		assert_eq!(edited, r"{\clip(3,m 0 0 b 28 32 40 48 64 64 l 0 0)}x");
		assert_eq!(vektor_yaz(source, "clip", &points[..4], true), source);
	}

	#[test]
	fn pos_oku_yaz_sil() {
		let m = "{\\pos(10,20)}merhaba";
		assert_eq!(etiket_al(m, "pos").as_deref(), Some("(10,20)"));
		assert_eq!(
			etiket_yaz(m, "pos", "(30,40)"),
			"{\\pos(30,40)}merhaba"
		);
		assert_eq!(etiket_sil(m, "pos"), "{}merhaba");
		assert_eq!(etiket_sil(m, "fscx"), m);
	}

	#[test]
	fn etiketsiz_metne_blok_acar() {
		assert_eq!(
			etiket_yaz("merhaba", "fscx", "120"),
			"{\\fscx120}merhaba"
		);
		assert_eq!(etiket_al("merhaba", "fscx"), None);
		assert_eq!(etiket_sil("merhaba", "fscx"), "merhaba");
	}

	#[test]
	fn cok_bloklu_metin_korunur() {
		let m = "{\\fscx120}a{\\fscy90}b";
		assert_eq!(etiket_al(m, "fscx").as_deref(), Some("120"));
		assert_eq!(
			etiket_yaz(m, "pos", "(1,2)"),
			"{\\fscx120\\pos(1,2)}a{\\fscy90}b"
		);
		assert_eq!(etiket_sil(m, "fscx"), "{}a{\\fscy90}b");
	}

	#[test]
	fn sayilar_ve_olcek() {
		assert_eq!(sayi_al("120"), Some(120.0));
		assert_eq!(sayi_al("(10,20)"), Some(10.0));
		assert_eq!(sayi_al("-45.5"), Some(-45.5));
		assert_eq!(sayi_al("m 0 0"), Some(0.0));
		assert_eq!(sayi_al("yok"), None);
		assert_eq!(
			etiket_yaz("{\\fscx120\\fscy90}x", "fscx", "150"),
			"{\\fscx150\\fscy90}x"
		);
	}

	#[test]
	fn dikdortgen_clip_ve_vektor() {
		let m = "{\\clip(1,2,3,4)}x";
		assert_eq!(etiket_al(m, "clip").as_deref(), Some("(1,2,3,4)"));
		assert_eq!(vektor_noktalari(m, "clip"), None);

		let v = "{\\clip(m 0 0 l 10 0 10 10)}x";
		assert_eq!(
			vektor_noktalari(v, "clip"),
			Some(vec![(0.0, 0.0), (10.0, 0.0), (10.0, 10.0)])
		);
		assert_eq!(
			vektor_yaz(v, "clip", &[(0.0, 0.0), (10.0, 0.0), (10.0, 10.0)], true),
			"{\\clip(m 0 0 l 10 0 l 10 10 l 0 0)}x"
		);
		assert_eq!(vektor_yaz(m, "clip", &[], false), "{}x");
	}

	#[test]
	fn t_donusumu_ve_bozuk_blok() {
		let m = "{\\t(\\fscx150)\\pos(1,2)}x";
		assert_eq!(etiket_al(m, "fscx"), None);
		assert_eq!(etiket_al(m, "pos").as_deref(), Some("(1,2)"));
		assert_eq!(
			etiket_yaz("{\\pos(1,2)merhaba", "fscx", "90"),
			"{\\fscx90}{\\pos(1,2)merhaba"
		);
	}
}
