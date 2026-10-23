//! Ses paneli görünür aralık matematiği — egui/ABI bağımsız saf fonksiyonlar.
//!
//! `Gorunum::d0` görünür aralığın ilk örneği, `adet` görünür örnek sayısı,
//! `gen` ise çizim genişliğidir (piksel). Tüm dönüşümler görünür aralığa
//! kırpılır; ara çarpımlar `i128` ile taşmadan hesaplanır.

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Gorunum {
	pub d0: i64,
	pub adet: i64,
	pub gen: f32,
}

/// Görünür aralığın geçerli olup olmadığını denetler: genişlik pozitif,
/// görünür örnek sayısı da pozitif olmalı.
fn gecerli(g: &Gorunum) -> bool {
	g.gen.is_finite() && g.gen > 0.0 && g.adet > 0
}

/// Piksel → görünür aralık içindeki örnek. Aralık dışı x ve geçersiz
/// görünüm `d0` döndürür; sonuç `d0..=d0+adet` aralığına kırpılır.
pub fn px_to_ornek(x: f32, g: &Gorunum) -> i64 {
	if !gecerli(g) || !x.is_finite() {
		return g.d0;
	}
	let x = x.clamp(0.0, g.gen) as f64;
	let gen = g.gen as f64;
	let adet = g.adet as i128;
	let pay = (x / gen * adet as f64).round() as i128;
	(g.d0 as i128 + pay).clamp(i64::MIN as i128, i64::MAX as i128) as i64
}

/// Görünür aralık içindeki örnek → piksel. Aralık dışı örnek en yakın
/// kenara kırpılır; geçersiz görünüm 0 döndürür.
pub fn ornek_to_px(s: i64, g: &Gorunum) -> f32 {
	if !gecerli(g) {
		return 0.0;
	}
	let alt = g.d0 as i128;
	let ust = alt + g.adet as i128;
	let s = (s as i128).clamp(alt, ust);
	let px = (s - alt) as f64 / g.adet as f64 * g.gen as f64;
	px.clamp(0.0, g.gen as f64) as f32
}

/// Görünür aralığı `capa_px` altındaki örneği sabit tutarak büyüt/küçült.
/// Yeni görünür örnek sayısı `[min(256, toplam), toplam]` aralığına,
/// `d0` ise `[0, toplam-adet]` aralığına kırpılır.
pub fn zoom_merkez(g: &mut Gorunum, carpan: f64, capa_px: f32, toplam: i64) {
	if !g.gen.is_finite() || g.gen <= 0.0 {
		return;
	}
	if toplam <= 0 {
		g.d0 = 0;
		g.adet = g.adet.max(1);
		return;
	}
	if !carpan.is_finite() || carpan <= 0.0 {
		return;
	}
	let eski_adet = g.adet.clamp(1, toplam);
	let alt_sinir = 256i64.min(toplam);
	let yeni_adet = ((eski_adet as f64 / carpan).round() as i64)
		.clamp(alt_sinir, toplam);
	let capa = if capa_px.is_finite() {
		capa_px.clamp(0.0, g.gen)
	} else {
		g.gen * 0.5
	};
	let oran = capa as f64 / g.gen as f64;
	let merkez = g.d0 as i128
		+ (oran * eski_adet as f64).round() as i128;
	let yeni_d0 = merkez
		- (oran * yeni_adet as f64).round() as i128;
	let maks = toplam - yeni_adet;
	g.adet = yeni_adet;
	g.d0 = yeni_d0.clamp(0, maks as i128) as i64;
}

/// Görünür aralığı yatay olarak kaydır. Pozitif `dx_px` içeriği sağa
/// taşır, yani daha eski örnekleri gösterir. Sonuç sarmaz; kenarda durur.
pub fn kaydir(g: &mut Gorunum, dx_px: f32, toplam: i64) {
	if !g.gen.is_finite() || g.gen <= 0.0 {
		return;
	}
	if toplam <= 0 {
		g.d0 = 0;
		return;
	}
	let adet = g.adet.clamp(1, toplam);
	let dx = if dx_px.is_finite() { dx_px } else { 0.0 };
	let delta = (dx as f64 / g.gen as f64 * adet as f64).round() as i128;
	let maks = toplam - adet;
	let yeni = g.d0 as i128 - delta;
	g.adet = adet;
	g.d0 = yeni.clamp(0, maks as i128) as i64;
}

/// İki seçim ucunu küçük/büyük sırasına diz.
pub fn secim_normalize(a: i64, b: i64) -> (i64, i64) {
	if a <= b { (a, b) } else { (b, a) }
}

/// Örneği en yakın video karesi sınırına oturt: `round(ornek / adım) * adım`
/// (`adım = rate/fps`). `rate <= 0` veya `fps <= 0` ise girdi aynen döner.
pub fn kareye_oturt(ornek: i64, rate: i32, fps: f64) -> i64 {
	if rate <= 0 || !fps.is_finite() || fps <= 0.0 {
		return ornek;
	}
	let adim = rate as f64 / fps;
	if !adim.is_finite() || adim <= 0.0 {
		return ornek;
	}
	let kare = (ornek as f64 / adim).round();
	let sinir = kare * adim;
	if !sinir.is_finite() {
		return ornek;
	}
	sinir.round() as i64
}

/// `audio_info` dizgesinden gerçek örnekleme hızı ve kanal sayısını çıkar.
/// ABI biçimi: `"s16 48000Hz 2k 12345 örnek (0.26 sn)"` (kanal alanı `%dk`).
/// Eksik/bozuk alanlarda 48000 Hz / 2 kanal varsayılanı kullanılır.
pub fn ses_kunye_coz(info: &str) -> (i32, i32) {
	let mut rate = 48_000;
	let mut ch = 2;
	for parca in info.split_whitespace() {
		if let Some(v) = parca.strip_suffix("Hz") {
			if let Ok(r) = v.parse::<i32>() {
				if r > 0 {
					rate = r;
				}
			}
			continue;
		}
		if let Some(v) = parca.strip_suffix("ch").or_else(|| parca.strip_suffix('k')) {
			if let Ok(c) = v.parse::<i32>() {
				if (1..=64).contains(&c) {
					ch = c;
				}
			}
		}
	}
	(rate, ch)
}

#[cfg(test)]
mod testler {
	use super::*;

	#[test]
	fn sifir_genislik_ve_bos_aralik() {
		let g = Gorunum { d0: 100, adet: 1000, gen: 0.0 };
		assert_eq!(px_to_ornek(50.0, &g), 100);
		assert_eq!(ornek_to_px(500, &g), 0.0);

		let negatif = Gorunum { d0: 100, adet: 1000, gen: -4.0 };
		assert_eq!(px_to_ornek(50.0, &negatif), 100);
		assert_eq!(ornek_to_px(500, &negatif), 0.0);

		let bos = Gorunum { d0: 100, adet: 0, gen: 100.0 };
		assert_eq!(px_to_ornek(50.0, &bos), 100);
		assert_eq!(ornek_to_px(150, &bos), 0.0);
	}

	#[test]
	fn negatif_dx_ve_kaydirma_siniri() {
		let mut g = Gorunum { d0: 1000, adet: 1000, gen: 1000.0 };
		// İçerik sağa sürüklenir → görünür aralık geriye gider.
		kaydir(&mut g, -100.0, 5000);
		assert_eq!(g.d0, 1100);
		kaydir(&mut g, 100.0, 5000);
		assert_eq!(g.d0, 1000);
		// Sınırlar: başa ve sona yapışır, sarmaz.
		kaydir(&mut g, 9999.0, 5000);
		assert_eq!(g.d0, 0);
		kaydir(&mut g, -9999.0, 5000);
		assert_eq!(g.d0, 4000);
		// Boş ses: d0 sıfırda kalır.
		let mut bos = Gorunum { d0: 7, adet: 10, gen: 100.0 };
		kaydir(&mut bos, -50.0, 0);
		assert_eq!(bos.d0, 0);
	}

	#[test]
	fn zoom_min_max_ve_capa_sabit() {
		let mut g = Gorunum { d0: 4000, adet: 2000, gen: 1000.0 };
		// %50'deki örnek 5000; 2× yakınlaşınca 1000 örnek görünür
		// ve aynı örnek yine %50'de kalır.
		zoom_merkez(&mut g, 2.0, 500.0, 10_000);
		assert_eq!(g.adet, 1000);
		assert_eq!(g.d0, 4500);

		// Aşırı yakınlaşma alt sınırda (256) durur.
		zoom_merkez(&mut g, 1000.0, 0.0, 10_000);
		assert_eq!(g.adet, 256);
		assert_eq!(g.d0, 4500);

		// Aşırı uzaklaşma tüm sesi kapsar; d0 sıfıra kırpılır.
		zoom_merkez(&mut g, 0.001, 0.0, 10_000);
		assert_eq!(g.adet, 10_000);
		assert_eq!(g.d0, 0);

		// Toplam 256'dan küçükse üst sınır toplamdır.
		let mut k = Gorunum { d0: 0, adet: 100, gen: 500.0 };
		zoom_merkez(&mut k, 2.0, 250.0, 100);
		assert_eq!(k.adet, 100);
		assert_eq!(k.d0, 0);
	}

	#[test]
	fn secim_ters_yon() {
		assert_eq!(secim_normalize(500, 100), (100, 500));
		assert_eq!(secim_normalize(100, 500), (100, 500));
		assert_eq!(secim_normalize(42, 42), (42, 42));
		assert_eq!(secim_normalize(-5, 5), (-5, 5));
	}

	#[test]
	fn kare_yuvarlama() {
		// 25 fps @ 48 kHz → kare adımı 1920 örnek.
		assert_eq!(kareye_oturt(950, 48_000, 25.0), 0);
		assert_eq!(kareye_oturt(970, 48_000, 25.0), 1920);
		assert_eq!(kareye_oturt(1000, 48_000, 25.0), 1920);
		assert_eq!(kareye_oturt(2879, 48_000, 25.0), 1920);
		assert_eq!(kareye_oturt(2881, 48_000, 25.0), 3840);
		assert_eq!(kareye_oturt(-1000, 48_000, 25.0), -1920);
		// Geçersiz hız/fps girdiyi aynen döndürür.
		assert_eq!(kareye_oturt(1234, 0, 25.0), 1234);
		assert_eq!(kareye_oturt(1234, 48_000, 0.0), 1234);
		assert_eq!(kareye_oturt(1234, -48_000, -25.0), 1234);
	}

	#[test]
	fn tasma_ve_sinirlar() {
		let g = Gorunum { d0: i64::MAX - 5, adet: 10, gen: 100.0 };
		assert_eq!(px_to_ornek(100.0, &g), i64::MAX);
		// d0 = MAX-5, adet = 10 → MAX örneği görünür aralığın ortası.
		assert_eq!(ornek_to_px(i64::MAX, &g), 50.0);
		assert_eq!(ornek_to_px(i64::MIN, &g), 0.0);
		assert_eq!(px_to_ornek(f32::NAN, &g), g.d0);

		let genis = Gorunum { d0: 0, adet: i64::MAX, gen: 100.0 };
		assert_eq!(ornek_to_px(i64::MAX, &genis), 100.0);
		assert_eq!(px_to_ornek(100.0, &genis), i64::MAX);
	}

	#[test]
	fn ses_kunye_cozme() {
		assert_eq!(
			ses_kunye_coz("s16 44100Hz 1k 44100 örnek (1.00 sn)"),
			(44_100, 1)
		);
		assert_eq!(
			ses_kunye_coz("s16 48000Hz 2k 96000 örnek (2.00 sn)"),
			(48_000, 2)
		);
		assert_eq!(ses_kunye_coz("bozuk künye"), (48_000, 2));
		assert_eq!(ses_kunye_coz("s16 0Hz 0k"), (48_000, 2));
	}
}
