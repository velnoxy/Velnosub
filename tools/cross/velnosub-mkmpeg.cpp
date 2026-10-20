// Aegisub Project http://www.aegisub.org/
//
// velnosub Faz 3 (P3.1 yardımcı) — test-varlık üreticisi: sentetik videoyu
// kendi statik FFmpeg'imizin mpeg4 kodlayıcısıyla mp4 konteynerine yazar.
// Cihazda video-probe dumanının kullanıcı medyasına muhtaç olmaması için.
//
// Kullanım: velnosub-mkmpeg <çıkış.mp4> [--saniye N] [--w W] [--h H]
// Marka şeridi: [MK0] konteyner, [MK1] kodlayıcı, [MK2] kare sayısı,
// [MK3] tamam, [MKE] hata.

extern "C" {
#include <libavcodec/avcodec.h>
#include <libavformat/avformat.h>
#include <libavutil/opt.h>
}

#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <iostream>
#include <string>

namespace {
void doldur(const AVFrame *f, int w, int h, int kare) {
	// Y: yatay merdiven + kayan dikey şerit; U/V: kareyle döngüleyen renk
	uint8_t *y = f->data[0];
	for (int j = 0; j < h; ++j) {
		for (int i = 0; i < w; ++i) {
			int v = i * 255 / (w > 1 ? w - 1 : 1);
			if (((i / 8) + kare) % 32 < 4)
				v = 210;
			y[j * f->linesize[0] + i] = (uint8_t)v;
		}
	}
	int cw = w / 2, ch = h / 2;
	uint8_t u = (uint8_t)((kare * 12) % 256 - 128);
	uint8_t v = (uint8_t)(200 - (kare * 12) % 256);
	for (int j = 0; j < ch; ++j) {
		std::memset(f->data[1] + j * f->linesize[1], u, cw);
		std::memset(f->data[2] + j * f->linesize[2], v, cw);
	}
}
} // namespace

int main(int argc, char **argv) {
	if (argc < 2) {
		std::cerr << "kullanım: velnosub-mkmpeg <çıkış.mp4> [--saniye N] [--w W] [--h H]\n";
		return 2;
	}
	std::string out = argv[1];
	int sn = 2, w = 320, h = 240;
	for (int i = 2; i < argc; ++i) {
		std::string a = argv[i];
		if (a == "--saniye" && i + 1 < argc) { sn = std::atoi(argv[++i]); }
		else if (a == "--w" && i + 1 < argc) { w = std::atoi(argv[++i]); }
		else if (a == "--h" && i + 1 < argc) { h = std::atoi(argv[++i]); }
	}
	if (sn <= 0 || sn > 60 || w <= 0 || h <= 0 || w % 2 || h % 2 || w > 1920 || h > 1080) {
		std::cerr << "[MKE] geçersiz süre/boyut\n";
		return 1;
	}

	const int fps = 24;
	AVFormatContext *oc = nullptr;
	if (avformat_alloc_output_context2(&oc, nullptr, "mp4", out.c_str()) < 0 || !oc) {
		std::cerr << "[MKE] konteyner oluşturulamadı\n";
		return 1;
	}
	std::cout << "[MK0] konteyner: " << oc->oformat->name << "\n";

	const AVCodec *cod = avcodec_find_encoder(AV_CODEC_ID_MPEG4);
	if (!cod) {
		std::cerr << "[MKE] mpeg4 kodlayıcı yok\n";
		return 1;
	}
	AVCodecContext *cc = avcodec_alloc_context3(cod);
	cc->codec_type = AVMEDIA_TYPE_VIDEO;
	cc->width = w;
	cc->height = h;
	cc->pix_fmt = AV_PIX_FMT_YUV420P;
	cc->time_base = {1, fps};
	cc->framerate = {fps, 1};
	cc->gop_size = 12;
	cc->max_b_frames = 0;
	if (oc->oformat->flags & AVFMT_GLOBALHEADER)
		cc->flags |= AV_CODEC_FLAG_GLOBAL_HEADER;
	if (avcodec_open2(cc, cod, nullptr) < 0) {
		std::cerr << "[MKE] kodlayıcı açılamadı\n";
		return 1;
	}
	std::cout << "[MK1] kodlayıcı: " << cod->name << " " << w << "x" << h
	          << " " << fps << "fps\n";

	AVStream *st = avformat_new_stream(oc, nullptr);
	if (!st) {
		std::cerr << "[MKE] akış oluşturulamadı\n";
		return 1;
	}
	st->time_base = {1, fps};
	if (avcodec_parameters_from_context(st->codecpar, cc) < 0) {
		std::cerr << "[MKE] parametreler kopyalanamadı\n";
		return 1;
	}

	if (!(oc->oformat->flags & AVFMT_NOFILE)) {
		if (avio_open(&oc->pb, out.c_str(), AVIO_FLAG_WRITE) < 0) {
			std::cerr << "[MKE] dosya açılamadı: " << out << "\n";
			return 1;
		}
	}
	if (avformat_write_header(oc, nullptr) < 0) {
		std::cerr << "[MKE] başlık yazılamadı\n";
		return 1;
	}

	AVFrame *f = av_frame_alloc();
	f->format = cc->pix_fmt;
	f->width = w;
	f->height = h;
	if (av_frame_get_buffer(f, 32) < 0) {
		std::cerr << "[MKE] kare tamponu yok\n";
		return 1;
	}

	int toplam = sn * fps, yazilan = 0;
	for (int k = 0; k < toplam; ++k) {
		if (av_frame_make_writable(f) < 0) break;
		doldur(f, w, h, k);
		f->pts = k;
		if (avcodec_send_frame(cc, f) < 0) break;
		AVPacket *pk = av_packet_alloc();
		while (avcodec_receive_packet(cc, pk) >= 0) {
			av_packet_rescale_ts(pk, cc->time_base, st->time_base);
			pk->stream_index = st->index;
			if (av_interleaved_write_frame(oc, pk) < 0) {
				av_packet_free(&pk);
				std::cerr << "[MKE] paket yazılamadı\n";
				return 1;
			}
			++yazilan;
			av_packet_unref(pk);
		}
		av_packet_free(&pk);
	}
	// kodlayıcıyı akıt
	avcodec_send_frame(cc, nullptr);
	AVPacket *pk = av_packet_alloc();
	while (avcodec_receive_packet(cc, pk) >= 0) {
		av_packet_rescale_ts(pk, cc->time_base, st->time_base);
		pk->stream_index = st->index;
		av_interleaved_write_frame(oc, pk);
		av_packet_unref(pk);
		++yazilan;
	}
	av_packet_free(&pk);
	av_write_trailer(oc);
	av_frame_free(&f);
	avcodec_free_context(&cc);
	if (!(oc->oformat->flags & AVFMT_NOFILE))
		avio_closep(&oc->pb);
	avformat_free_context(oc);

	std::cout << "[MK2] kare: " << yazilan << " paket (" << sn << " sn)\n";
	if (yazilan < toplam) {
		std::cerr << "[MKE] beklenenden az paket\n";
		return 1;
	}
	std::cout << "[MK3] tamam\n";
	return 0;
}
