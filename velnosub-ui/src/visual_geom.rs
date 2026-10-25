//! Typesetting koordinat eşlemesi — ASS PlayRes uzayı ↔ ekran/video alanı.
//!
//! Görüntü en-boyu korunur ve `Alan` içinde ortalanır. `Alan` video karesinin
//! ekranda kapladığı dikdörtgendir; letterbox boşluğu çağıran tarafta bu
//! dikdörtgene dahil edilmez.

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Alan {
	pub x: f32,
	pub y: f32,
	pub w: f32,
	pub h: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Esleme {
	/// En-boy koruyan eşlemede tek ölçek; doldurmada x/y'nin küçüğü.
	pub olcek: f32,
	pub olcek_x: f32,
	pub olcek_y: f32,
	pub ofs_x: f32,
	pub ofs_y: f32,
}

fn sonlu(v: f32, varsayilan: f32) -> f32 {
	if v.is_finite() { v } else { varsayilan }
}

/// ASS çözünürlüğünü `alan` içine en-boyu koruyarak sığdır.
pub fn esleme(ass_w: i32, ass_h: i32, alan: Alan) -> Esleme {
	let aw = ass_w.max(1) as f32;
	let ah = ass_h.max(1) as f32;
	let x = sonlu(alan.x, 0.0);
	let y = sonlu(alan.y, 0.0);
	let w = sonlu(alan.w, 1.0).max(0.0);
	let h = sonlu(alan.h, 1.0).max(0.0);
	let olcek = (w / aw).min(h / ah).max(0.0);
	Esleme {
		olcek,
		olcek_x: olcek,
		olcek_y: olcek,
		ofs_x: x + (w - aw * olcek) * 0.5,
		ofs_y: y + (h - ah * olcek) * 0.5,
	}
}

/// ASS çözünürlüğünü `alan`a **doldur** (libass kare ölçeği gibi).
/// PlayRes ile video en-boyu farklıysa x/y ayrı ölçeklenir; typeset
/// overlay'i render_line çıktısıyla birebir hizalanır.
pub fn esleme_doldur(ass_w: i32, ass_h: i32, alan: Alan) -> Esleme {
	let aw = ass_w.max(1) as f32;
	let ah = ass_h.max(1) as f32;
	let x = sonlu(alan.x, 0.0);
	let y = sonlu(alan.y, 0.0);
	let w = sonlu(alan.w, 1.0).max(0.0);
	let h = sonlu(alan.h, 1.0).max(0.0);
	let ox = (w / aw).max(0.0);
	let oy = (h / ah).max(0.0);
	Esleme {
		olcek: ox.min(oy),
		olcek_x: ox,
		olcek_y: oy,
		ofs_x: x,
		ofs_y: y,
	}
}

/// ASS noktası → ekran noktası.
pub fn ass_to_ekran(x: f64, y: f64, e: &Esleme) -> (f32, f32) {
	(
		(x as f32) * e.olcek_x + e.ofs_x,
		(y as f32) * e.olcek_y + e.ofs_y,
	)
}

/// Ekran noktası → ASS noktası. Ölçek sıfırsa (0,0).
pub fn ekran_to_ass(x: f32, y: f32, e: &Esleme) -> (f64, f64) {
	if !e.olcek_x.is_finite()
		|| !e.olcek_y.is_finite()
		|| e.olcek_x <= 0.0
		|| e.olcek_y <= 0.0
	{
		return (0.0, 0.0);
	}
	(
		((x - e.ofs_x) / e.olcek_x) as f64,
		((y - e.ofs_y) / e.olcek_y) as f64,
	)
}

/// İki ekran noktası `yaricap` kadar yakın mı? Tutamaç isabeti.
pub fn tutamac(a: (f32, f32), b: (f32, f32), yaricap: f32) -> bool {
	let dx = a.0 - b.0;
	let dy = a.1 - b.1;
	if !dx.is_finite() || !dy.is_finite() || !yaricap.is_finite() {
		return false;
	}
	let r = yaricap.max(0.0);
	dx * dx + dy * dy <= r * r
}

#[cfg(test)]
mod testler {
	use super::*;

	fn yakin(a: f32, b: f32) {
		assert!((a - b).abs() < 0.01, "beklenen {b}, gelen {a}");
	}

	#[test]
	fn letterbox_16_9_4_3_alanda() {
		let e = esleme(
			1920,
			1080,
			Alan { x: 0.0, y: 0.0, w: 800.0, h: 600.0 },
		);
		yakin(e.olcek, 800.0 / 1920.0);
		yakin(e.ofs_x, 0.0);
		yakin(e.ofs_y, 75.0);
		let (x, y) = ass_to_ekran(1920.0, 1080.0, &e);
		yakin(x, 800.0);
		yakin(y, 525.0);
	}

	#[test]
	fn kare_video_genis_alanda() {
		let e = esleme(
			1000,
			1000,
			Alan { x: 20.0, y: 10.0, w: 200.0, h: 100.0 },
		);
		yakin(e.olcek, 0.1);
		yakin(e.ofs_x, 70.0);
		yakin(e.ofs_y, 10.0);
	}

	#[test]
	fn playres_video_kutusuna_doldurulur() {
		let e = esleme_doldur(
			384,
			288,
			Alan { x: 0.0, y: 0.0, w: 1920.0, h: 1080.0 },
		);
		yakin(e.olcek_x, 5.0);
		yakin(e.olcek_y, 3.75);
		let (x, y) = ass_to_ekran(384.0, 288.0, &e);
		yakin(x, 1920.0);
		yakin(y, 1080.0);
		let (ax, ay) = ekran_to_ass(960.0, 540.0, &e);
		yakin(ax as f32, 192.0);
		yakin(ay as f32, 144.0);
	}

	#[test]
	fn ters_donusum() {
		let e = esleme(
			1280,
			720,
			Alan { x: 40.0, y: 30.0, w: 640.0, h: 360.0 },
		);
		let (sx, sy) = ass_to_ekran(320.0, 180.0, &e);
		let (ax, ay) = ekran_to_ass(sx, sy, &e);
		yakin(ax as f32, 320.0);
		yakin(ay as f32, 180.0);
	}

	#[test]
	fn tutamac_yaricapi() {
		assert!(tutamac((10.0, 10.0), (12.0, 14.0), 5.0));
		assert!(!tutamac((10.0, 10.0), (20.0, 20.0), 5.0));
		assert!(tutamac((0.0, 0.0), (0.0, 0.0), 0.0));
		assert!(!tutamac((f32::NAN, 0.0), (0.0, 0.0), 5.0));
	}

	#[test]
	fn gecersiz_olculer_cokmez() {
		let e = esleme(0, -5, Alan { x: 0.0, y: 0.0, w: 0.0, h: 0.0 });
		assert_eq!(e.olcek, 0.0);
		assert_eq!(ekran_to_ass(10.0, 10.0, &e), (0.0, 0.0));
	}
}
