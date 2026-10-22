//! Petit générateur pseudo-aléatoire écrit à la main (SplitMix64) : aucune dépendance, et la
//! même graine rend toujours la même suite, sur toute machine.

/// Générateur SplitMix64. Pas cryptographique : il ne sert qu'à varier les exemples.
#[derive(Debug, Clone)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed)
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Entier dans `0..n` (`n` > 0). Le léger biais du modulo est sans effet ici.
    pub fn below(&mut self, n: usize) -> usize {
        assert!(n > 0, "below(0)");
        (self.next_u64() % n as u64) as usize
    }

    /// Entier dans `lo..=hi`.
    pub fn range(&mut self, lo: u64, hi: u64) -> u64 {
        lo + self.next_u64() % (hi - lo + 1)
    }

    /// Vrai avec la probabilité `num / den`.
    pub fn chance(&mut self, num: u64, den: u64) -> bool {
        self.next_u64() % den < num
    }

    pub fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.below(items.len())]
    }

    /// Mélange de Fisher-Yates.
    pub fn shuffle<T>(&mut self, items: &mut [T]) {
        for i in (1..items.len()).rev() {
            let j = self.below(i + 1);
            items.swap(i, j);
        }
    }

    /// `len` chiffres hexadécimaux minuscules.
    pub fn hex(&mut self, len: usize) -> String {
        (0..len)
            .map(|_| char::from(b"0123456789abcdef"[self.below(16)]))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_same_seed_gives_the_same_sequence() {
        let (mut a, mut b) = (Rng::new(7), Rng::new(7));
        let xs: Vec<u64> = (0..100).map(|_| a.next_u64()).collect();
        let ys: Vec<u64> = (0..100).map(|_| b.next_u64()).collect();
        assert_eq!(xs, ys);
        assert_ne!(xs[0], Rng::new(8).next_u64());
    }

    #[test]
    fn the_known_first_value_does_not_drift() {
        // Valeur de référence de SplitMix64 pour la graine 0 : garde le format reproductible.
        assert_eq!(Rng::new(0).next_u64(), 0xE220_A839_7B1D_CDAF);
    }

    #[test]
    fn bounds_are_respected() {
        let mut r = Rng::new(1);
        for _ in 0..1000 {
            assert!(r.below(3) < 3);
            let x = r.range(5, 9);
            assert!((5..=9).contains(&x));
        }
        let mut v: Vec<u32> = (0..20).collect();
        r.shuffle(&mut v);
        let mut sorted = v.clone();
        sorted.sort();
        assert_eq!(sorted, (0..20).collect::<Vec<_>>());
        assert_eq!(r.hex(8).len(), 8);
    }
}
