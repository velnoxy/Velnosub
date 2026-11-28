#[derive(Clone, Copy, Default)]
pub(crate) struct Options {
	pub lead_in: i64,
	pub lead_out: i64,
	pub gap: i64,
	pub overlap: i64,
	pub bias: u8,
	pub start_before_key: i64,
	pub start_after_key: i64,
	pub end_before_key: i64,
	pub end_after_key: i64,
}

fn snap(time: i64, keys: &[i64], before: i64, after: i64) -> i64 {
	let index = keys.partition_point(|&key| key <= time);
	let earlier = index.checked_sub(1).and_then(|i| keys.get(i)).copied();
	let later = keys.get(index).copied();
	match (earlier.filter(|&key| time - key <= after), later.filter(|&key| key - time <= before)) {
		(Some(a), Some(b)) if b - time < time - a => b,
		(Some(a), _) => a,
		(_, Some(b)) => b,
		_ => time,
	}
}

/// Lines carry original row indices. Key timestamps are in ascending order.
pub(crate) fn process(lines: &mut [(usize, [i64; 2])], cfg: Options, keys: &[i64]) {
	lines.sort_by_key(|(_, [start, _])| *start);
	if cfg.lead_in > 0 {
		for index in 0..lines.len() {
			let [start, _] = lines[index].1;
			let edge = lines[..index].iter().map(|(_, [_, end])| *end)
				.filter(|&end| end <= start).max().unwrap_or(0);
			lines[index].1[0] = (start - cfg.lead_in).max(edge).max(0);
		}
	}
	if cfg.lead_out > 0 {
		for index in 0..lines.len() {
			let [_, end] = lines[index].1;
			let edge = lines[index + 1..].iter().map(|(_, [start, _])| *start)
				.filter(|&start| start >= end).min().unwrap_or(i64::MAX);
			lines[index].1[1] = end.saturating_add(cfg.lead_out).min(edge);
		}
	}
	for index in 1..lines.len() {
		let [previous_start, previous_end] = lines[index - 1].1;
		let [start, end] = lines[index].1;
		let distance = start - previous_end;
		if (distance > 0 && distance <= cfg.gap) || (distance < 0 && -distance <= cfg.overlap) {
			let join = previous_end + distance * i64::from(cfg.bias) / 100;
			if join > previous_start && join < end {
				lines[index - 1].1[1] = join;
				lines[index].1[0] = join;
			}
		}
	}
	for (_, times) in lines {
		let [start, end] = *times;
		let next_start = snap(start, keys, cfg.start_before_key, cfg.start_after_key);
		let next_end = snap(end, keys, cfg.end_before_key, cfg.end_after_key);
		if next_start < next_end {
			*times = [next_start, next_end];
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn extends_joins_and_snaps_using_real_key_timestamps() {
		let mut lines = [(2, [1060, 1300]), (0, [800, 1000])];
		let options = Options {
			lead_out: 20, gap: 50, bias: 50, start_before_key: 12, end_after_key: 25,
			..Options::default()
		};
		process(&mut lines, options, &[805, 1301]);
		assert_eq!(lines, [(0, [805, 1040]), (2, [1040, 1301])]);
	}

	#[test]
	fn does_not_invert_overlapping_or_very_short_lines() {
		let mut lines = [(0, [0, 1000]), (1, [510, 520])];
		let options = Options { overlap: 500, bias: 50, start_before_key: 6, end_after_key: 6, ..Options::default() };
		process(&mut lines, options, &[515]);
		assert_eq!(lines, [(0, [0, 1000]), (1, [510, 520])]);
	}
}
