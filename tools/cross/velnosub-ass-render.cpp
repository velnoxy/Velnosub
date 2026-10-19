// velnosub Faz 2 (P2.3) — libass kare render dumanı.
//
// velnosub-core.so'ya bağlanır (libass satırıyla); headless/fıkırlar:
//   - fontconfig YOK (ASS_FONTPROVIDER_NONE); font ass_add_font ile yayınlanır.
//   - Çakan çıktı PPM P6 (bağımsız denetim: siyah-kompozit + glyph kaplaması).
//
// Kullanım:
//   velnosub-ass-render <giriş.ass> <çıkış.ppm> [--font <ttf>] [
//       --t <saniye>] [--w <px>] [--h <px>] [--family <font adı>]
//
// Marker anlamları:
//   [AR0] libass kütüphanesi kuruldu
//   [AR1] ASS yüklandı  (olaylar: <N>)
//   [AR2] kare render edildi (glyph pikseli: <N>)
//   [AR3] PPM yazıldı   (<bayt>)
//   [ARE] hata

#include <ass/ass.h>

#include <cstdint>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <string>
#include <vector>

namespace {

struct Args {
	std::string in_path;
	std::string out_path;
	std::string font_path;
	std::string family = "Default";
	double t = 1.0;
	int w = 640;
	int h = 360;
};

bool read_file(std::string const &path, std::vector<char> &out) {
	FILE *f = fopen(path.c_str(), "rb");
	if (!f) return false;
	fseek(f, 0, SEEK_END);
	long size = ftell(f);
	fseek(f, 0, SEEK_SET);
	if (size < 0) { fclose(f); return false; }
	out.resize(static_cast<size_t>(size));
	size_t got = fread(out.data(), 1, out.size(), f);
	fclose(f);
	return got == out.size();
}

void log_msg(std::string const &s) { fprintf(stderr, "%s\n", s.c_str()); }

// ASS_Image bitmaşı kapsama (coverage) alfa maskesidir (0..255, 255 = tam);
// renk 0xAARRGGBB ve AA byte'ı 0 = tam opak. İkisinin çarpımı etkiyi verir.
unsigned long compose_frame(ASS_Image *head, std::vector<uint32_t> &buf,
                            int w, int h) {
	unsigned long glyphs = 0;
	for (ASS_Image *img = head; img; img = img->next) {
		if (!img->bitmap || img->w <= 0 || img->h <= 0) continue;
		uint32_t color = img->color;
		unsigned alpha = (color >> 24) & 0xFF;          // 0 opak, 255 saydam
		unsigned r = (color >> 16) & 0xFF;
		unsigned g = (color >> 8) & 0xFF;
		unsigned b = color & 0xFF;
		for (int yy = 0; yy < img->h; ++yy) {
			int py = img->dst_y + yy;
			if (py < 0 || py >= h) continue;
			unsigned char const *row = img->bitmap + yy * img->stride;
			for (int xx = 0; xx < img->w; ++xx) {
				int px = img->dst_x + xx;
				if (px < 0 || px >= w) continue;
				unsigned cover = row[xx];
				if (!cover) continue;
				// etki = coverage * (255 - global alfa) / 255
				unsigned eff = cover * (255u - alpha) / 255u;
				if (!eff) continue;
				++glyphs;
				uint32_t &dst = buf[static_cast<size_t>(py) * w + px];
				unsigned dr = (dst >> 16) & 0xFF, dg = (dst >> 8) & 0xFF,
				         db = dst & 0xFF;
				dr = (r * eff + dr * (255u - eff)) / 255u;
				dg = (g * eff + dg * (255u - eff)) / 255u;
				db = (b * eff + db * (255u - eff)) / 255u;
				dst = 0xFF000000u | (dr << 16) | (dg << 8) | db;
			}
		}
	}
	return glyphs;
}

bool write_ppm(std::string const &path, std::vector<uint32_t> const &buf,
               int w, int h) {
	FILE *f = fopen(path.c_str(), "wb");
	if (!f) return false;
	fprintf(f, "P6\n%d %d\n255\n", w, h);
	std::vector<unsigned char> line(static_cast<size_t>(w) * 3);
	for (int y = 0; y < h; ++y) {
		for (int x = 0; x < w; ++x) {
			uint32_t px = buf[static_cast<size_t>(y) * w + x];
			line[static_cast<size_t>(x) * 3 + 0] = (px >> 16) & 0xFF;
			line[static_cast<size_t>(x) * 3 + 1] = (px >> 8) & 0xFF;
			line[static_cast<size_t>(x) * 3 + 2] = px & 0xFF;
		}
		if (fwrite(line.data(), 1, line.size(), f) != line.size()) {
			fclose(f);
			return false;
		}
	}
	fclose(f);
	return true;
}

}  // namespace

int main(int argc, char **argv) {
	Args a;
	if (argc < 3) {
		fprintf(stderr,
		        "kullanım: velnosub-ass-render <giriş.ass> <çıkış.ppm> "
		        "[--font ttf] [--t sn] [--w px] [--h px] [--family ad]\n");
		return 2;
	}
	a.in_path = argv[1];
	a.out_path = argv[2];
	for (int i = 3; i < argc; ++i) {
		std::string opt = argv[i];
		auto val = [&](std::string &dst) {
			if (i + 1 >= argc) return false;
			dst = argv[++i];
			return true;
		};
		if (opt == "--font") { if (!val(a.font_path)) return 2; }
		else if (opt == "--t") { if (i + 1 < argc) a.t = atof(argv[++i]); }
		else if (opt == "--w") { if (i + 1 < argc) a.w = atoi(argv[++i]); }
		else if (opt == "--h") { if (i + 1 < argc) a.h = atoi(argv[++i]); }
		else if (opt == "--family") { if (!val(a.family)) return 2; }
		else { log_msg(std::string("[ARE] bilinmeyen seçenek: ") + opt); return 2; }
	}
	if (a.w <= 0 || a.h <= 0 || a.w > 4096 || a.h > 4096) {
		log_msg("[ARE] --w/--h aralık dışı");
		return 2;
	}

	ASS_Library *lib = ass_library_init();
	if (!lib) { log_msg("[ARE] ass_library_init bitti boş"); return 1; }

	if (!a.font_path.empty()) {
		std::vector<char> fdata;
		if (!read_file(a.font_path, fdata)) {
			log_msg("[ARE] font dosyası okunamadı: " + a.font_path);
			ass_library_done(lib);
			return 1;
		}
		ass_add_font(lib, a.family.data(), fdata.data(),
		             static_cast<int>(fdata.size()));
	}
	log_msg("[AR0] libass kütühtanesi kuruldu");

	std::vector<char> script;
	if (!read_file(a.in_path, script)) {
		log_msg("[ARE] ASS okunamadı: " + a.in_path);
		ass_library_done(lib);
		return 1;
	}
	ASS_Track *track = ass_new_track(lib);
	ass_process_data(track, script.data(), static_cast<int>(script.size()));
	log_msg(std::string("[AR1] ASS yüklendi (olaylar: ") +
	        std::to_string(track->n_events) + ")");
	if (track->n_events <= 0) {
		log_msg("[ARE] ASS olay listesi boş");
		ass_free_track(track);
		ass_library_done(lib);
		return 1;
	}

	ASS_Renderer *renderer = ass_renderer_init(lib);
	if (!renderer) {
		log_msg("[ARE] ass_renderer_init bitti boş");
		ass_free_track(track);
		ass_library_done(lib);
		return 1;
	}
	ass_set_frame_size(renderer, a.w, a.h);
	ass_set_fonts(renderer, nullptr, nullptr, ASS_FONTPROVIDER_NONE, nullptr, 1);

	int change = 0;
	ASS_Image *imgs = ass_render_frame(renderer, track, a.t * 1000.0, &change);
	std::vector<uint32_t> buf(static_cast<size_t>(a.w) * a.h, 0u);
	unsigned long glyphs = compose_frame(imgs, buf, a.w, a.h);
	log_msg(std::string("[AR2] kare render edildi (glyph pikseli: ") +
	        std::to_string(glyphs) + ")");
	ass_free_track(track);
	ass_renderer_done(renderer);

	bool ok = glyphs > 0 && write_ppm(a.out_path, buf, a.w, a.h);
	ass_library_done(lib);
	if (!ok) { log_msg("[ARE] PPM yazılamadı"); return 1; }
	log_msg(std::string("[AR3] PPM yazıldı (") + a.out_path + ")");
	return 0;
}
