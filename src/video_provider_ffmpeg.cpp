// Copyright (c) 2010-2013, Thomas Goyne <plorkyeran@aegisub.org>
//
// Permission to use, copy, modify, and distribute this software for any
// purpose with or without fee is hereby granted, provided that the above
// copyright notice and this permission notice appear in all copies.
//
// THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES
// WITH REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF
// MERCHANTABILITY AND FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR
// ANY SPECIAL, DIRECT, INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES
// WHATSOEVER RESULTING FROM LOSS OF USE, DATA OR PROFITS, WHETHER IN AN
// ACTION OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION, ARISING OUT OF
// OR IN CONNECTION WITH THE USE OR PERFORMANCE OF THIS SOFTWARE.
//
// Aegisub Project http://www.aegisub.org/

/// @file video_provider_ffmpeg.cpp
/// @brief velnosub Faz 3 (P3.2) — ffmpeg tabanlı video sağlayıcı (mobil kol)
/// @ingroup video_input
///
/// FFmpeg 6.1 statik kitaplıklarıyla (build-ffmpeg.sh çıktısı) kare-doğru
/// okuma: PTS tablosu birikimi (ts tabanlı geri sarma) + tek kare önbellek.
/// Aegisub çekirdek arayüzüne (AGI_CORE) wx'siz derlenir; masa kolunda bu
/// dosya hiç derlenmez (-Dvelnosub_mobile kapısı).

#ifdef WITH_VELNOSUB_FFMPEG

extern "C" {
#include <libavcodec/avcodec.h>
#include <libavformat/avformat.h>
#include <libavutil/pixdesc.h>
#include <libswscale/swscale.h>
}

#include "include/aegisub/video_provider.h"
#include "video_frame.h"

#include <libaegisub/fs.h>
#include <libaegisub/vfr.h>

#include <algorithm>
#include <cstdint>
#include <memory>
#include <string>
#include <vector>

namespace agi { class BackgroundRunner; }

namespace {
class FFmpegVideoProvider final : public VideoProvider {
	/// Konteyner ve kod akışı
	AVFormatContext *fmt = nullptr;
	AVCodecContext *codec_ctx = nullptr;
	int stream_idx = -1;
	int width = 0;
	int height = 0;
	double dar = 0.0;
	agi::vfr::Framerate fps;

	/// ycbcr → BGRA ölçekleyici
	SwsContext *scaler = nullptr;

	/// arama durumu: her karenin PTS'i (akış time_base) sırayla birikir
	std::vector<int64_t> frame_pts;
	/// decoder konumu: son çözümlü karenin numarası (-1 = hiç çözülmedi)
	int cur_frame_no = -1;
	/// frame_buf'un karşılığı (tek karelik önbellek)
	int buf_no = -1;
	std::vector<unsigned char> frame_buf;
	AVFrame *cur_frame = nullptr;

	/// bir sonraki çözümlü kareyi al (parser döngüsü); true = kare hazır
	bool step_decode(int64_t &pts_out) {
		bool got = false;
		while (!got) {
			int rr = avcodec_receive_frame(codec_ctx, cur_frame);
			if (rr == 0) {
				got = true;
				pts_out = cur_frame->best_effort_timestamp != AV_NOPTS_VALUE
				              ? cur_frame->best_effort_timestamp
				              : cur_frame->pts;
				break;
			}
			if (rr == AVERROR_EOF) break;
			if (rr != AVERROR(EAGAIN))
				throw VideoDecodeError("ffmpeg video provider: kod çözme hatası");

			AVPacket *pkt = av_packet_alloc();
			if (!pkt)
				throw VideoDecodeError("ffmpeg video provider: paket tamponu yok");
			rr = av_read_frame(fmt, pkt);
			if (rr < 0) {
				av_packet_free(&pkt);
				if (avcodec_send_packet(codec_ctx, nullptr) >= 0) continue;
				return false; // son
			}
			if (pkt->stream_index == stream_idx && avcodec_send_packet(codec_ctx, pkt) < 0) {
				av_packet_free(&pkt);
				throw VideoDecodeError("ffmpeg video provider: paket gönderilemedi");
			}
			av_packet_free(&pkt);
		}
		return true;
	}

	/// frame_buf'i güncel dec hattından BGRA'ya ölçekle ve önbelleğe işle
	void fill_cache(VideoFrame &frame) {
		uint8_t *dst[4] = {frame_buf.empty() ? nullptr : &frame_buf[0], nullptr, nullptr, nullptr};
		int dst_stride[4] = {width * 4, 0, 0, 0};
		sws_scale(scaler, cur_frame->data, cur_frame->linesize, 0, height, dst, dst_stride);
		frame.data.resize(frame_buf.size());
		std::copy(frame_buf.begin(), frame_buf.end(), frame.data.begin());
		frame.width = width;
		frame.height = height;
		frame.pitch = (size_t)width * 4;
		frame.flipped = false;
		buf_no = cur_frame_no;
	}

public:
	FFmpegVideoProvider(agi::fs::path const& filename) {
		avformat_network_init();

		std::string path = filename.generic_string();
		if (avformat_open_input(&fmt, path.c_str(), nullptr, nullptr) < 0)
			throw VideoNotSupported("ffmpeg video provider: dosya açılamadı: " + path);
		if (avformat_find_stream_info(fmt, nullptr) < 0)
			throw VideoOpenError("ffmpeg video provider: akış bilgisi yok: " + path);

		const AVCodec *codec = nullptr;
		int rc = av_find_best_stream(fmt, AVMEDIA_TYPE_VIDEO, -1, -1, &codec, 0);
		if (rc < 0 || !codec)
			throw VideoNotSupported("ffmpeg video provider: video akışı yok: " + path);
		stream_idx = rc;
		AVStream *st = fmt->streams[stream_idx];

		codec_ctx = avcodec_alloc_context3(codec);
		if (!codec_ctx || avcodec_parameters_to_context(codec_ctx, st->codecpar) < 0 ||
		    avcodec_open2(codec_ctx, codec, nullptr) < 0)
			throw VideoOpenError("ffmpeg video provider: codec zinciri kurulamadı");

		width = codec_ctx->width;
		height = codec_ctx->height;
		if (width <= 0 || height <= 0)
			throw VideoOpenError("ffmpeg video provider: geçersiz çözünürlük");

		AVRational r_fps = av_guess_frame_rate(fmt, st, nullptr);
		if (r_fps.num <= 0 || r_fps.den <= 0)
			r_fps = st->avg_frame_rate;
		if (r_fps.num <= 0 || r_fps.den <= 0)
			throw VideoOpenError("ffmpeg video provider: kare hızı çözümlenemedi");
		fps = agi::vfr::Framerate(r_fps.num, r_fps.den);

		AVRational sar = st->sample_aspect_ratio;
		dar = sar.num > 0 && sar.den > 0
		          ? (double)width * sar.num / ((double)height * sar.den)
		          : (double)width / height;

		// BGRA ölçekleyici — VideoFrame verisi bgra8 düzende (video_frame.h)
		scaler = sws_getContext(width, height, codec_ctx->pix_fmt,
		                        width, height, AV_PIX_FMT_BGRA,
		                        SWS_BICUBIC, nullptr, nullptr, nullptr);
		if (!scaler)
			throw VideoOpenError("ffmpeg video provider: swscale kurulamadı");

		cur_frame = av_frame_alloc();
		frame_buf.resize((size_t)width * height * 4);
	}

	~FFmpegVideoProvider() {
		if (cur_frame) av_frame_free(&cur_frame);
		if (scaler) sws_freeContext(scaler);
		if (codec_ctx) avcodec_free_context(&codec_ctx);
		if (fmt) avformat_close_input(&fmt);
	}

	void GetFrame(int n, VideoFrame &frame) override {
		if (buf_no == n) {
			// tek kare önbellek isabeti
			frame.data.resize(frame_buf.size());
			std::copy(frame_buf.begin(), frame_buf.end(), frame.data.begin());
			frame.width = width;
			frame.height = height;
			frame.pitch = (size_t)width * 4;
			frame.flipped = false;
			return;
		}

		int pos = cur_frame_no;

		// geri sarma: birikmiş PTS tablosundan ts'i hedefle
		if (pos >= n) {
			if (n < 0 || n >= (int)frame_pts.size())
				throw VideoDecodeError("ffmpeg video provider: geri sarma için PTS kaydı yok (kare " + std::to_string(n) + ")");
			int64_t target = frame_pts[n];
			avcodec_flush_buffers(codec_ctx);
			if (av_seek_frame(fmt, stream_idx, target, AVSEEK_FLAG_BACKWARD) < 0)
				throw VideoDecodeError("ffmpeg video provider: arama başarısız (kare " + std::to_string(n) + ")");
			avcodec_flush_buffers(codec_ctx);
			// hedef PTS'e kadar çöz (ek kareleri sayma); kare hedefe geldiğinde dur
			for (int guard = 0;; ++guard) {
				int64_t pts;
				if (!step_decode(pts))
					throw VideoDecodeError("ffmpeg video provider: arama sonrası kare bitti");
				if (pts >= target || guard >= 512) break;
			}
			pos = n;
		}

		while (pos < n) {
			int64_t pts;
			if (!step_decode(pts))
				throw VideoDecodeError("ffmpeg video provider: kare bitti (istenen " + std::to_string(n) + ")");
			++pos;
			if (pos >= (int)frame_pts.size())
				frame_pts.push_back(pts);
		}
		cur_frame_no = pos;

		fill_cache(frame);
	}

	void SetColorSpace(std::string const& matrix) override { (void)matrix; }

	int GetFrameCount() const override {
		// PTS tablosu tamamen birikmediyse süre × fps kestirimi kullanılır
		if (fmt->duration > 0 && fps.FPS() > 0.0)
			return (int)((fmt->duration / (double)AV_TIME_BASE) * fps.FPS() + 0.5);
		return 1;
	}

	int GetWidth() const override { return width; }
	int GetHeight() const override { return height; }
	double GetDAR() const override { return dar; }
	agi::vfr::Framerate GetFPS() const override { return fps; }

	std::vector<int> GetKeyFrames() const override {
		// P3.2: keyframe tablosu bu sağlayıcıda henüz üretilmez (dahili keyframe
		// üretimi ayrı adım); boş liste düşme riski sıfır.
		return {};
	}

	std::string GetColorSpace() const override {
		AVStream *st = fmt->streams[stream_idx];
		int cs = (int)st->codecpar->color_space;
		int cr = st->codecpar->color_range == AVCOL_RANGE_JPEG ? AGI_CR_JPEG : AGI_CR_MPEG;
		return ColorMatrix::colormatrix_description(cs, cr);
	}

	std::string GetDecoderName() const override { return "FFmpeg (velnosub)"; }

	bool HasAudio() const override {
		for (unsigned i = 0; i < fmt->nb_streams; ++i)
			if (fmt->streams[i]->codecpar->codec_type == AVMEDIA_TYPE_AUDIO)
				return true;
		return false;
	}
};
} // namespace

std::unique_ptr<VideoProvider> CreateFFmpegVideoProvider(agi::fs::path const& filename, std::string const& colormatrix, agi::BackgroundRunner *) {
	return std::unique_ptr<VideoProvider>(new FFmpegVideoProvider(filename));
}

#endif // WITH_VELNOSUB_FFMPEG
