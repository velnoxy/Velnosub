//! Aegisub yerleşim matematiği (P9.1).
//!
//! Çerçeve seçimi ve üst çalışma alanının bölmeleri burada saf fonksiyon
//! olarak durur; `abi.rs` yalnız sonucu çizer. Böylece ekran boyutu / video
//! oranı kuralları GUI açmadan test edilir.

/// Etkin çerçeve: yatay ve en az 800 pt genişlik Aegisub geometrisi,
/// kalan her durum (portre/dar yatay) mobil sekmeler.
#[derive(PartialEq, Clone, Copy, Debug)]
pub enum Yerlesim {
	Aegisub,
	Mobil,
}

/// Üst çalışma alanının bölmeleri (menü + araç çubuğu hariç).
#[derive(PartialEq, Clone, Copy, Debug)]
pub struct Bolmeler {
	pub ust_yuk: f32,
	pub video_gen: f32,
	pub sag_gen: f32,
	pub ses_yuk: f32,
	pub duzen_yuk: f32,
}

/// Ekran oranına göre çerçeve seçimi: `gen > yuk && gen >= 800`.
pub fn sec(gen: f32, yuk: f32) -> Yerlesim {
	if gen > yuk && gen >= 800.0 {
		Yerlesim::Aegisub
	} else {
		Yerlesim::Mobil
	}
}

/// Zoom'lu egui noktalarını yerel (taban ppp) boyuta indirger.
/// `taban` geçersizse oran uygulanmaz.
pub fn taban_boyut(gen: f32, yuk: f32, ppp: f32, taban: f32) -> (f32, f32) {
	let oran = if taban > 0.0 { ppp / taban } else { 1.0 };
	(gen * oran, yuk * oran)
}

/// Yön kararını iki eksende de zoom'dan bağımsız verir; yalnız genişliği
/// normalize etmek 1280×1024 gibi ekranlarda yatay/dikey oranı bozuyordu.
pub fn sec_taban(gen: f32, yuk: f32, ppp: f32, taban: f32) -> Yerlesim {
	let (gen, yuk) = taban_boyut(gen, yuk, ppp, taban);
	sec(gen, yuk)
}

/// Aegisub üst çalışma alanı: yüksekliğin %55'i; solda video, sağda ses
/// (%32) + düzen kutusu. Video genişliği video oranından türetilir
/// (oran yoksa 16:9) ve ekran genişliğinin %32–%52 aralığına kırpılır.
pub fn bolmeler(gen: f32, yuk: f32, video_oran: Option<f32>) -> Bolmeler {
	bolmeler_ust(gen, yuk * 0.55, video_oran)
}

/// Üst çalışma alanı yüksekliği zaten biliniyorsa (yeniden
/// boyutlandırılabilir panel) aynı bölmeleri doğrudan üretir.
pub(crate) fn bolmeler_ust(
	gen: f32,
	ust_yuk: f32,
	video_oran: Option<f32>,
) -> Bolmeler {
	let oran = video_oran.unwrap_or(16.0 / 9.0);
	let video_gen = (ust_yuk * oran).clamp(gen * 0.32, gen * 0.52);
	let sag_gen = gen - video_gen;
	let ses_yuk = ust_yuk * 0.32;
	let duzen_yuk = ust_yuk - ses_yuk;
	Bolmeler {
		ust_yuk,
		video_gen,
		sag_gen,
		ses_yuk,
		duzen_yuk,
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	/// Kayan nokta karşılaştırması: bölme sonuçları 0.5 pt'den yakın olmalı.
	fn yakin(a: f32, b: f32, ne: &str) {
		assert!(
			(a - b).abs() < 0.5,
			"{ne}: beklenen {b}, gelen {a}"
		);
	}

	#[test]
	fn secim_portre_mobil() {
		assert_eq!(sec(1080.0, 2400.0), Yerlesim::Mobil);
	}

	#[test]
	fn secim_kare_mobil() {
		// genişlik yükseklikten büyük değil → mobil
		assert_eq!(sec(800.0, 800.0), Yerlesim::Mobil);
	}

	#[test]
	fn secim_dar_yatay_mobil() {
		// yatay ama 800 pt eşiğinin altında → mobil
		assert_eq!(sec(700.0, 400.0), Yerlesim::Mobil);
	}

	#[test]
	fn secim_2400x1080_aegisub() {
		assert_eq!(sec(2400.0, 1080.0), Yerlesim::Aegisub);
	}

	#[test]
	fn secim_2560x1600_aegisub() {
		assert_eq!(sec(2560.0, 1600.0), Yerlesim::Aegisub);
	}

	#[test]
	fn bolmeler_2400x1080_varsayilan_16_9() {
		let b = bolmeler(2400.0, 1080.0, None);
		yakin(b.ust_yuk, 594.0, "ust_yuk");
		yakin(b.video_gen, 1056.0, "video_gen");
		yakin(b.sag_gen, 1344.0, "sag_gen");
		yakin(b.ses_yuk, 190.08, "ses_yuk");
		yakin(b.duzen_yuk, 403.92, "duzen_yuk");
	}

	#[test]
	fn bolmeler_2560x1600_ust_sinir() {
		let b = bolmeler(2560.0, 1600.0, None);
		yakin(b.ust_yuk, 880.0, "ust_yuk");
		// 16:9 adayı 1564.4; genişliğin %52 üst sınırı 1331.2'ye kırpılır
		yakin(b.video_gen, 1331.2, "video_gen");
		yakin(b.sag_gen, 1228.8, "sag_gen");
		yakin(b.ses_yuk, 281.6, "ses_yuk");
		yakin(b.duzen_yuk, 598.4, "duzen_yuk");
	}

	#[test]
	fn bolmeler_alt_sinir() {
		let b = bolmeler(2400.0, 1080.0, Some(0.1));
		yakin(b.ust_yuk, 594.0, "ust_yuk");
		// 594 × 0.1 = 59.4; genişliğin %32 alt sınırı 768'e kırpılır
		yakin(b.video_gen, 768.0, "video_gen");
		yakin(b.sag_gen, 1632.0, "sag_gen");
	}

	#[test]
	fn bolmeler_ust_dogrudan_esit() {
		// `bolmeler(gen, 800)` üst yüksekliği 440 üretir; panel yeniden
		// boyutlandırıldığında kullanılan doğrudan yol aynı sonucu vermeli.
		let gen = 1280.0;
		let dolayli = bolmeler(gen, 800.0, Some(2.0));
		let dogrudan = bolmeler_ust(gen, 440.0, Some(2.0));
		yakin(dogrudan.ust_yuk, dolayli.ust_yuk, "ust_yuk");
		yakin(dogrudan.video_gen, dolayli.video_gen, "video_gen");
		yakin(dogrudan.sag_gen, dolayli.sag_gen, "sag_gen");
		yakin(dogrudan.ses_yuk, dolayli.ses_yuk, "ses_yuk");
		yakin(dogrudan.duzen_yuk, dolayli.duzen_yuk, "duzen_yuk");
	}

	#[test]
	fn bolmeler_4_3_video() {
		let b = bolmeler(2560.0, 1600.0, Some(4.0 / 3.0));
		yakin(b.ust_yuk, 880.0, "ust_yuk");
		// 880 × 4/3 = 1173.33; alt/üst sınırların arasında kalır
		yakin(b.video_gen, 1173.333, "video_gen");
		yakin(b.sag_gen, 1386.667, "sag_gen");
		yakin(b.ses_yuk, 281.6, "ses_yuk");
		yakin(b.duzen_yuk, 598.4, "duzen_yuk");
	}

	#[test]
	fn bolmeler_none_16_9_ile_ayni() {
		let yok = bolmeler(2400.0, 1080.0, None);
		let onalti = bolmeler(2400.0, 1080.0, Some(16.0 / 9.0));
		yakin(yok.video_gen, onalti.video_gen, "video_gen");
		yakin(yok.sag_gen, onalti.sag_gen, "sag_gen");
	}

	#[test]
	fn bolmeler_toplam_korunur() {
		let b = bolmeler(1280.0, 800.0, Some(2.0));
		yakin(b.video_gen + b.sag_gen, 1280.0, "genişlik toplamı");
		yakin(b.ses_yuk + b.duzen_yuk, b.ust_yuk, "yükseklik toplamı");
	}

	#[test]
	fn sec_taban_zoomdan_bagimsiz() {
		// 1280×1024 yerel ekran, 0.8× zoom: egui noktaları 1600×1280.
		// İki eksen de taban ppp'ye indirgenince yatay oran korunur.
		let (gen, yuk) = taban_boyut(1600.0, 1280.0, 0.8, 1.0);
		yakin(gen, 1280.0, "taban genişlik");
		yakin(yuk, 1024.0, "taban yükseklik");
		assert_eq!(
			sec_taban(1600.0, 1280.0, 0.8, 1.0),
			Yerlesim::Aegisub
		);
	}

	#[test]
	fn taban_boyut_sifir_taban_korur() {
		// Savunma: taban ppp 0/negatifse oran uygulanmaz.
		assert_eq!(taban_boyut(100.0, 50.0, 2.0, 0.0), (100.0, 50.0));
	}
}
