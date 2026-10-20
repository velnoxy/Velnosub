// Aegisub Project http://www.aegisub.org/
//
// velnosub Faz 3 (P3.4) — oboe oynatma dumanı: audio-probe'un yazdığı s16 PCM
// dosyasını Oboe (AAudio/OpenSLES geri uçları) ile çalar. Doğrudan geri
// çağrım pompası; rate/kanal argümanlarla.
//
// Cihaz gerektirir (emülatörde gerçek çıkış yok); masa kolunda hedef bilinçli
// atlanır.
//
// Kullanım: velnosub-audio-play <dosya.pcm> [--rate N] [--ch N]
// Marker şeridi: [OP0] akış açıldı (rate, kanal, tampon), [OP1] tampon yüklü,
// [OP2] koşum raporu, [OP3] tamam, [OPE] hata.

#include <oboe/Oboe.h>

#include <chrono>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <iostream>
#include <string>
#include <thread>
#include <vector>

namespace {
class PCMPlayer final : public oboe::AudioStreamDataCallback, public oboe::AudioStreamErrorCallback {
public:
	PCMPlayer(std::vector<int16_t> samples, int rate, int chs)
	: pcm(std::move(samples)), sample_rate(rate), channels(chs) {}

	oboe::DataCallbackResult onAudioReady(oboe::AudioStream *stream, void *data, int32_t num_frames) override {
		(void)stream;
		auto *out = static_cast<int16_t *>(data);
		for (int32_t i = 0; i < num_frames; ++i) {
			for (int c = 0; c < channels; ++c) {
				if (cursor < (int)pcm.size())
					out[i * channels + c] = pcm[cursor++];
				else
					out[i * channels + c] = 0; // boşalma
			}
		}
		if (cursor >= (int)pcm.size())
			drained = true;
		if (drained)
			return oboe::DataCallbackResult::Stop;
		return oboe::DataCallbackResult::Continue;
	}

	void onErrorAfterClose(oboe::AudioStream *stream, oboe::Result error) override {
		(void)stream;
		std::cerr << "[OPE] akış hatası: " << oboe::convertToText(error) << "\n";
		defect = true;
	}

	bool run() {
		oboe::AudioStreamBuilder builder;
		builder.setDirection(oboe::Direction::Output)
		    ->setPerformanceMode(oboe::PerformanceMode::LowLatency)
		    ->setSharingMode(oboe::SharingMode::Shared) // cihaz Exclusive'e düşebilir
		    ->setFormat(oboe::AudioFormat::I16)
		    ->setChannelCount(channels)
		    ->setSampleRate(sample_rate)
		    ->setDataCallback(this)
		    ->setErrorCallback(this);

		std::shared_ptr<oboe::AudioStream> stream;
		oboe::Result rc = builder.openStream(stream);
		if (rc != oboe::Result::OK) {
			std::cerr << "[OPE] akış açılamadı: " << oboe::convertToText(rc) << "\n";
			return false;
		}
		std::cout << "[OP0] akış açıldı: rate " << stream->getSampleRate() << ", ch "
		          << stream->getChannelCount() << ", buffer "
		          << stream->getBufferSizeInFrames() << " frames\n";

		rc = stream->start();
		if (rc != oboe::Result::OK) {
			std::cerr << "[OPE] başlatılamadı: " << oboe::convertToText(rc) << "\n";
			stream->close();
			return false;
		}

		// boşalma bekleyişi: süre + payıra payı
		double secs = pcm.size() / (double)(sample_rate * channels);
		int loops = (int)(secs * 100.0) + 200;
		int loop = 0;
		while (!drained && !defect && loop < loops) {
			std::this_thread::sleep_for(std::chrono::milliseconds(10));
			++loop;
		}
		stream->stop();
		stream->close();
		if (defect) return false;
		std::cout << "[OP2] koşum: " << cursor << " sample oynatıldı / süre " << secs
		          << " sn\n";
		return true;
	}

	std::vector<int16_t> pcm;
	int sample_rate = 48000;
	int channels = 2;
	int64_t cursor = 0;
	bool drained = false;
	volatile bool defect = false;
};
} // namespace

int main(int argc, char **argv) {
	if (argc < 2) {
		std::cerr << "kullanım: velnosub-audio-play <dosya.pcm> [--rate N] [--ch N]\n";
		return 2;
	}
	std::string path = argv[1];
	int rate = 48000;
	int chs = 2;
	for (int i = 2; i < argc; ++i) {
		std::string a = argv[i];
		if (a == "--rate" && i + 1 < argc) {
			rate = std::atoi(argv[i + 1]);
			++i;
		} else if (a == "--ch" && i + 1 < argc) {
			chs = std::atoi(argv[i + 1]);
			++i;
		}
	}
	if (rate <= 0 || chs <= 0 || chs > 8) {
		std::cerr << "[OPE] geçersiz rate/ch\n";
		return 1;
	}

	std::FILE *f = std::fopen(path.c_str(), "rb");
	if (!f) {
		std::cerr << "[OPE] dosya açılmadı: " << path << "\n";
		return 1;
	}
	std::vector<int16_t> pcm;
	int16_t buf[4096];
	size_t n;
	while ((n = std::fread(buf, sizeof(int16_t), 4096, f)) > 0)
		pcm.insert(pcm.end(), buf, buf + n);
	std::fclose(f);
	if (pcm.empty()) {
		std::cerr << "[OPE] PCM veri yok\n";
		return 1;
	}
	double secs = pcm.size() / (double)(rate * chs);
	std::cout << "[OP1] tampon yüklü: " << pcm.size() << " sample / " << secs
	          << " sn (rate " << rate << ", ch " << chs << ")\n";

	PCMPlayer player(std::move(pcm), rate, chs);
	bool ok = player.run();
	if (!ok) return 1;
	std::cout << "[OP3] tamam\n";
	return 0;
}
