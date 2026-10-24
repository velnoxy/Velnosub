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

/// `\clip`/`\iclip` değerini vektör noktalarına çöz. Dikdörtgen clip veya
/// yol komutu olmayan değer `None` döner.
pub fn vektor_noktalari(metin: &str, ad: &str) -> Option<Vec<(f64, f64)>> {
	let ham = etiket_al(metin, ad)?;
	let s = ham.trim();
	let s = s.strip_prefix('(').and_then(|x| x.strip_suffix(')')).unwrap_or(s);
	let mut komut: Option<char> = None;
	let mut sayilar: Vec<f64> = Vec::new();
	let mut noktalar: Vec<(f64, f64)> = Vec::new();
	for tok in s.split(|c: char| c == ',' || c.is_whitespace()) {
		if tok.is_empty() {
			continue;
		}
		let mut kc = tok.chars();
		if let (Some(c), None) = (kc.next(), kc.next()) {
			if c.is_ascii_alphabetic() {
				komut = Some(c.to_ascii_lowercase());
				sayilar.clear();
				continue;
			}
		}
		let Ok(v) = tok.parse::<f64>() else { continue };
		let Some(c) = komut else { continue };
		let cift = match c {
			'm' | 'l' => 1,
			'b' | 's' => 3,
			_ => 1,
		};
		sayilar.push(v);
		if sayilar.len() == cift * 2 {
			let x = sayilar[sayilar.len() - 2];
			let y = sayilar[sayilar.len() - 1];
			if matches!(c, 'm' | 'l' | 'b' | 's') {
				noktalar.push((x, y));
			}
			sayilar.clear();
		}
	}
	if noktalar.is_empty() { None } else { Some(noktalar) }
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
