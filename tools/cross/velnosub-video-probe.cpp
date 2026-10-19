// Aegisub Project http://www.aegisub.org/
//
// velnosub Faz 3 (P3.1) — video dumanı: dosyayı aç, akışı seç, ilk N kareyi
// decode et; çözünürlük/fps/kare sayısı/PTS markerlarıyla raporlar.
// Bağımsız ikilidir (çekirdek bağlantısı P3.2 sağlayıcı diliminde gelir).
//
// Kullanım: velnosub-video-probe <dosya> [--frames N]
// Marker şeridi: [VP0] açıldı (format, süre), [VP1] akış (codec, w*h, fps),
// [VP2] her karenin PTS'i, [VP3] decode kare adedi, [VPE] hata.

extern "C" {
#include <libavcodec/avcodec.h>
#include <libavformat/avformat.h>
#include <libavutil/avutil.h>
#include <libavutil/frame.h>
}

#include <cstdlib>
#include <iostream>
#include <string>

int main(int argc, char **argv) {
	if (argc < 2) {
		std::cerr << "kullanım: velnosub-video-probe <dosya> [--frames N]\n";
		return 2;
	}
	std::string path = argv[1];
	int want_frames = 5;
	for (int i = 2; i + 1 < argc; ++i) {
		if (std::string(argv[i]) == "--frames") {
			want_frames = std::atoi(argv[i + 1]);
			++i;
		}
	}

	AVFormatContext *fmt = nullptr;
	if (avformat_open_input(&fmt, path.c_str(), nullptr, nullptr) < 0 || !fmt) {
		std::cerr << "[VPE] açılamadı: " << path << "\n";
		return 1;
	}
	if (avformat_find_stream_info(fmt, nullptr) < 0) {
		std::cerr << "[VPE] akış bilgisi yok\n";
		avformat_close_input(&fmt);
		return 1;
	}
	const char *f_name = fmt->iformat ? fmt->iformat->long_name : "?";
	double duration = fmt->duration > 0 ? fmt->duration / (double)AV_TIME_BASE : 0.0;
	std::cout << "[VP0] açıldı: " << f_name
	          << " süre: " << duration << " sn\n";

	const AVCodec *codec = nullptr;
	int rc = av_find_best_stream(fmt, AVMEDIA_TYPE_VIDEO, -1, -1, &codec, 0);
	if (rc < 0 || !codec) {
		std::cerr << "[VPE] video akışı yok\n";
		avformat_close_input(&fmt);
		return 1;
	}
	int stream_idx = rc;
	AVStream *st = fmt->streams[stream_idx];
	AVCodecContext *ctx = avcodec_alloc_context3(codec);
	if (!ctx || avcodec_parameters_to_context(ctx, st->codecpar) < 0 ||
	    avcodec_open2(ctx, codec, nullptr) < 0) {
		std::cerr << "[VPE] codec zinciri kurulamadı\n";
		if (ctx) avcodec_free_context(&ctx);
		avformat_close_input(&fmt);
		return 1;
	}
	AVRational fps = av_guess_frame_rate(fmt, st, nullptr);
	double fps_d = fps.num > 0 && fps.den > 0 ? av_q2d(fps) : 0.0;
	std::cout << "[VP1] akış: " << codec->name
	          << " " << ctx->width << "x" << ctx->height
	          << " fps: " << fps_d << " (time_base "
	          << st->time_base.num << "/" << st->time_base.den << ")\n";

	AVPacket *pkt = av_packet_alloc();
	AVFrame *frm = av_frame_alloc();
	if (!pkt || !frm) {
		std::cerr << "[VPE] paket/kare tamponu yok\n";
		if (frm) av_frame_free(&frm);
		if (pkt) av_packet_free(&pkt);
		avcodec_free_context(&ctx);
		avformat_close_input(&fmt);
		return 1;
	}

	int decoded = 0;
	bool eof = false;
	bool ok = true;

	// besleme + süzme döngüsü: EAGAIN olana dek sür, sonra paket ver,
	// dosya sonunda send(nullptr) ile boşalt. PTS [VP2]; time_base st'den.
	AVRational tb = st->time_base;
	while (decoded < want_frames && !eof && ok) {
		int rr = avcodec_receive_frame(ctx, frm);
		if (rr == AVERROR(EAGAIN) || rr == AVERROR_EOF) {
			if (rr == AVERROR_EOF) eof = true;
		} else if (rr < 0) {
			ok = false;
			break;
		} else {
			++decoded;
			double pts_s = (tb.num > 0 && frm->pts != AV_NOPTS_VALUE)
			               ? frm->pts * av_q2d(tb) : 0.0;
			std::cout << "[VP2] kare " << decoded << " pts: " << pts_s << " sn\n";
			av_frame_unref(frm);
			if (decoded >= want_frames) break;
		}

		int sr = av_read_frame(fmt, pkt);
		if (sr < 0) {
			// dosya sonu: süzme fazı
			avcodec_send_packet(ctx, nullptr);
			while (decoded < want_frames) {
				rr = avcodec_receive_frame(ctx, frm);
				if (rr != 0) break;
				++decoded;
				double pts_s = (tb.num > 0 && frm->pts != AV_NOPTS_VALUE)
				               ? frm->pts * av_q2d(tb) : 0.0;
				std::cout << "[VP2] kare " << decoded << " pts: " << pts_s << " sn\n";
				av_frame_unref(frm);
			}
			break;
		}
		if (pkt->stream_index == stream_idx &&
		    avcodec_send_packet(ctx, pkt) < 0) {
			ok = false;
		}
		av_packet_unref(pkt);
	}

	std::cout << "[VP3] decode kareler: " << decoded << "\n";
	int out = (ok && decoded > 0) ? 0 : 1;

	av_frame_free(&frm);
	av_packet_free(&pkt);
	avcodec_free_context(&ctx);
	avformat_close_input(&fmt);
	return out;
}
