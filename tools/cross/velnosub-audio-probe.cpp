// Aegisub Project http://www.aegisub.org/
//
// velnosub Faz 3 (P3.3) — ses dumanı: dosyayı aç, ses akışını seç, swresample
// ile s16'ya bunniştir, PCM'i parça-parça diske yazar. Disk tabanlı PCM
// önbellek (P3.4 Oboe oynatma zemini) kanıtıdır.
//
// Host-check'te sistem ffmpeg yoksa meson hedefi bilinçli atlanır.
//
// Kullanım: velnosub-audio-probe <dosya> [--seconds N] [cikis.pcm]
// Marker şeridi: [AP0] açıldı (format, süre), [AP1] akış (codec, kanal, samplerate),
// [AP2] parça-parça yazım (her chunk satırın [AP2] gösterimi), [AP3] toplam,
// [APE] hata.

extern "C" {
#include <libavcodec/avcodec.h>
#include <libavformat/avformat.h>
#include <libavutil/avutil.h>
#include <libavutil/channel_layout.h>
#include <libavutil/frame.h>
#include <libavutil/samplefmt.h>
#include <libswresample/swresample.h>
}

#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <iostream>
#include <string>

int main(int argc, char **argv) {
	if (argc < 2) {
		std::cerr << "kullanım: velnosub-audio-probe <dosya> [--seconds N] [cikis.pcm]\n";
		return 2;
	}
	std::string path = argv[1];
	double want_seconds = 5.0;
	std::string outpath;
	for (int i = 2; i < argc; ++i) {
		std::string a = argv[i];
		if (a == "--seconds" && i + 1 < argc) {
			want_seconds = std::atof(argv[i + 1]);
			++i;
		} else if (!a.empty() && a[0] != '-') {
			outpath = a;
		}
	}
	if (outpath.empty()) {
		outpath = path + ".pcm";
	}

	avformat_network_init();
	AVFormatContext *fmt = nullptr;
	if (avformat_open_input(&fmt, path.c_str(), nullptr, nullptr) < 0) {
		std::cerr << "[APE] dosya açılmadı: " << path << "\n";
		return 1;
	}
	if (avformat_find_stream_info(fmt, nullptr) < 0) {
		std::cerr << "[APE] akış bilgisi yok\n";
		avformat_close_input(&fmt);
		return 1;
	}
	const char *f_name = fmt->iformat ? fmt->iformat->long_name : "?";
	double duration = fmt->duration > 0 ? fmt->duration / (double)AV_TIME_BASE : 0.0;
	std::cout << "[AP0] açıldı: " << f_name << " süre: " << duration << " sn\n";

	int rc = av_find_best_stream(fmt, AVMEDIA_TYPE_AUDIO, -1, -1, nullptr, 0);
	if (rc < 0) {
		std::cerr << "[APE] ses akışı yok\n";
		avformat_close_input(&fmt);
		return 1;
	}
	int stream_idx = rc;
	AVStream *st = fmt->streams[stream_idx];
	const AVCodec *codec = avcodec_find_decoder(st->codecpar->codec_id);
	if (!codec) {
		std::cerr << "[APE] decoder yok (kod id " << st->codecpar->codec_id << ")\n";
		avformat_close_input(&fmt);
		return 1;
	}
	AVCodecContext *ctx = avcodec_alloc_context3(codec);
	if (!ctx || avcodec_parameters_to_context(ctx, st->codecpar) < 0) {
		std::cerr << "[APE] codec bağlamı kurulamadı\n";
		if (ctx) avcodec_free_context(&ctx);
		avformat_close_input(&fmt);
		return 1;
	}
	if (avcodec_open2(ctx, codec, nullptr) < 0) {
		std::cerr << "[APE] decoder açılamadı\n";
		avcodec_free_context(&ctx);
		avformat_close_input(&fmt);
		return 1;
	}

	const AVChannelLayout *in_layout = &ctx->ch_layout;
	int out_channels = in_layout->nb_channels > 0 ? in_layout->nb_channels : 1;
	int in_sample_rate = ctx->sample_rate > 0 ? ctx->sample_rate : 48000;
	std::cout << "[AP1] akış: " << codec->name << " kanal: " << out_channels
	          << " samplerate: " << in_sample_rate << " (girdi "
	          << av_get_sample_fmt_name(ctx->sample_fmt) << ")\n";

	SwrContext *swr = nullptr;
	AVChannelLayout out_layout;
	av_channel_layout_default(&out_layout, out_channels);
	if (swr_alloc_set_opts2(&swr, &out_layout, AV_SAMPLE_FMT_S16, in_sample_rate,
	                        in_layout, ctx->sample_fmt, in_sample_rate, 0, nullptr) < 0) {
		std::cerr << "[APE] swr kurulamadı\n";
		avcodec_free_context(&ctx);
		avformat_close_input(&fmt);
		return 1;
	}
	if (swr_init(swr) < 0) {
		std::cerr << "[APE] swr init edilemedi\n";
		swr_free(&swr);
		avcodec_free_context(&ctx);
		avformat_close_input(&fmt);
		return 1;
	}

	std::FILE *out = std::fopen(outpath.c_str(), "wb");
	if (!out) {
		std::cerr << "[APE] çıkış açılamadı: " << outpath << "\n";
		swr_free(&swr);
		avcodec_free_context(&ctx);
		avformat_close_input(&fmt);
		return 1;
	}

	AVPacket *pkt = av_packet_alloc();
	AVFrame *frm = av_frame_alloc();
	if (!pkt || !frm) {
		std::cerr << "[APE] paket/kare tamponu yok\n";
		if (pkt) av_packet_free(&pkt);
		if (frm) av_frame_free(&frm);
		std::fclose(out);
		swr_free(&swr);
		avcodec_free_context(&ctx);
		avformat_close_input(&fmt);
		return 1;
	}

	int64_t want_samples = (int64_t)(want_seconds * in_sample_rate);
	int64_t total_samples = 0;
	int chunk = 0;
	bool eof = false;
	bool ok = true;

	while (total_samples < want_samples && !eof && ok) {
		int rr = avcodec_receive_frame(ctx, frm);
		if (rr == AVERROR(EAGAIN)) {
			rr = av_read_frame(fmt, pkt);
			if (rr < 0) {
				if (rr == AVERROR_EOF) {
					if (avcodec_send_packet(ctx, nullptr) >= 0) continue;
					eof = true;
					continue;
				} else if (rr != AVERROR(EAGAIN)) {
					break;
				}
			} else if (pkt->stream_index == stream_idx) {
				if (avcodec_send_packet(ctx, pkt) < 0) {
					ok = false;
					break;
				}
				av_packet_unref(pkt);
			} else {
				av_packet_unref(pkt);
			}
			continue;
		} else if (rr == AVERROR_EOF) {
			eof = true;
			continue;
		} else if (rr < 0) {
			ok = false;
			break;
		}

		uint8_t **dst_data = nullptr;
		int dst_linesize = 0;
		int max_out = frm->nb_samples > 0 ? frm->nb_samples : 1024;
		if (av_samples_alloc_array_and_samples(&dst_data, &dst_linesize, out_channels,
		                                       max_out, AV_SAMPLE_FMT_S16, 0) < 0) {
			ok = false;
			break;
		}
		int got = swr_convert(swr, dst_data, max_out, (const uint8_t **)frm->extended_data, frm->nb_samples);
		if (got < 0) {
			av_freep(&dst_data[0]);
			av_freep(&dst_data);
			ok = false;
			break;
		}
		if (got > 0) {
			int bytes = got * out_channels * 2; // s16 interleaved
			std::fwrite(dst_data[0], 1, (size_t)bytes, out);
			total_samples += got;
			++chunk;
			std::cout << "[AP2] parça " << chunk << ": " << got << " sample (toplam "
			          << total_samples << ")\n";
		}
		av_freep(&dst_data[0]);
		av_freep(&dst_data);
	}

	if (out) std::fclose(out);
	std::cout << "[AP3] toplam: " << total_samples << " sample / "
	          << total_samples * out_channels * 2 << " bayt / " << chunk << " parça → "
	          << outpath << "\n";

	av_frame_free(&frm);
	av_packet_free(&pkt);
	swr_free(&swr);
	av_channel_layout_uninit(&out_layout);
	avcodec_free_context(&ctx);
	avformat_close_input(&fmt);

	if (!ok) {
		std::cerr << "[APE] koşum başarıslandı\n";
		return 1;
	}
	std::cout << "[AP4] tamam\n";
	return 0;
}
