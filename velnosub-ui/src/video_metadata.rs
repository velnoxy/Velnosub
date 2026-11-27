fn rows(text: &str) -> (&str, impl Iterator<Item = &str>) {
    let mut lines = text.trim_start_matches('\u{feff}').lines().map(str::trim);
    let header = lines.next().unwrap_or("");
    (header, lines.filter(|line| !line.is_empty() && !line.starts_with('#')))
}

fn timestamp(value: f64) -> Result<i32, String> {
    let rounded = value.round();
    if (0.0..=f64::from(i32::MAX)).contains(&rounded) {
        Ok(rounded as i32)
    } else {
        Err("Kare zamanı geçersiz veya desteklenen sınırın dışında".into())
    }
}

fn fps(value: &str) -> Result<f64, String> {
    value.trim().parse::<f64>().ok()
        .filter(|fps| *fps > 0.0 && fps.is_finite())
        .ok_or_else(|| "Pozitif kare hızı bekleniyor".into())
}

pub(crate) fn timecodes(text: &str, count: usize) -> Result<Vec<i32>, String> {
    let (header, mut data) = rows(text);
    if header.eq_ignore_ascii_case("# timecode format v2") {
        let times = data.map(|line| {
            timestamp(line.parse().map_err(|_| "Kare zamanı okunamadı")?)
        }).collect::<Result<Vec<_>, String>>()?;
        if times.len() != count || times.windows(2).any(|pair| pair[0] > pair[1]) {
            return Err("Zamanlar sıralı olmalı ve video kare sayısıyla eşleşmeli".into());
        }
        return Ok(times);
    }
    if !header.eq_ignore_ascii_case("# timecode format v1") {
        return Err("Aegisub timecode v1 veya v2 dosyası bekleniyor".into());
    }

    let default = data.next().and_then(|line| line.split_once(char::is_whitespace))
        .filter(|(keyword, _)| keyword.eq_ignore_ascii_case("assume"))
        .ok_or("Timecode v1 dosyasında Assume satırı eksik")?;
    let mut rates = vec![fps(default.1)?; count];
    let mut next_start = 0;
    for line in data {
        let fields: Vec<_> = line.split(',').map(str::trim).collect();
        let [start, end, rate] = fields.as_slice() else {
            return Err("Başlangıç, bitiş ve kare hızı bekleniyor".into());
        };
        let start: usize = start.parse().map_err(|_| "Başlangıç karesi geçersiz")?;
        let end: usize = end.parse().map_err(|_| "Bitiş karesi geçersiz")?;
        if start < next_start || end < start || end >= count {
            return Err("Kare aralıkları sıralı olmalı, çakışmamalı ve videoya sığmalı".into());
        }
        rates[start..=end].fill(fps(rate)?);
        next_start = end + 1;
    }
    let mut ms = 0.0;
    rates.into_iter().map(|rate| {
        let start = timestamp(ms);
        ms += 1000.0 / rate;
        start
    }).collect()
}

pub(crate) fn keyframes(text: &str, count: i32) -> Result<Vec<i32>, String> {
    let (header, data) = rows(text);
    if !header.eq_ignore_ascii_case("# keyframe format v1") {
        return Err("Aegisub keyframe v1 dosyası bekleniyor".into());
    }
    let mut frames = Vec::new();
    for line in data {
        if let Some(value) = line.strip_prefix("fps ") {
            let rate = value.trim().parse::<f64>().map_err(|_| "Kare hızı okunamadı")?;
            if rate.is_finite() && rate >= 0.0 {
                continue;
            }
        }
        let frame = line.parse().map_err(|_| "Ana kare numarası okunamadı")?;
        if !(0..count).contains(&frame) {
            return Err("Ana kare video sınırının dışında".into());
        }
        frames.push(frame);
    }
    frames.sort_unstable();
    frames.dedup();
    Ok(frames)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_vfr_and_rejects_invalid_sequences() {
        assert_eq!(timecodes("# timecode format v2\n0\n24.5\n90", 3).unwrap(), [0, 25, 90]);
        assert!(timecodes("# timecode format v2\n0\n30\n20", 3).is_err());
        assert!(timecodes("# timecode format v2\n0\n20", 3).is_err());
    }

    #[test]
    fn v1_respects_overrides_and_rejects_overlap() {
        assert_eq!(timecodes("# timecode format v1\nAssume 25\n1,2,50", 4).unwrap(), [0, 40, 60, 80]);
        assert!(timecodes("# timecode format v1\nAssume 25\n1,2,50\n2,3,60", 4).is_err());
    }

    #[test]
    fn validates_keyframe_bounds() {
        assert_eq!(keyframes("# keyframe format v1\nfps 0\n4\n0\n4", 5).unwrap(), [0, 4]);
        assert!(keyframes("# keyframe format v1\n5", 5).is_err());
    }
}
