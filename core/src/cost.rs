//! Module de coût unique du noyau (AD-20, amendement du 2026-11-05), commun à
//! `terrain.cost_distance` et `terrain.least_cost_path`.
//!
//! Règles d'AD-20 : la valeur d'une cellule du raster de coût est un coût par mètre parcouru ;
//! une cellule sans donnée, non finie, nulle ou négative est infranchissable. Le déplacement se
//! fait aux 8 voisins ; un pas coûte la moyenne des coûts des deux cellules multipliée par la
//! distance entre leurs centres (taille de cellule en ligne ou en colonne, diagonale de la
//! cellule en biais, soit taille × √2 sur une cellule carrée). Un pas en biais n'est permis que
//! si les deux cellules qu'il frôle (ses voisines en ligne et en colonne) sont franchissables :
//! un mur de cellules infranchissables jointes par leurs coins arrête le déplacement (décision
//! du propriétaire, 2026-11-06).
//!
//! Méthode : algorithme de Dijkstra sur la grille, tas binaire à clé totale (coût cumulé, puis
//! index de la cellule), voisins parcourus dans un ordre fixe, prédécesseur changé seulement
//! sur un coût strictement plus petit : mêmes entrées, même grille, même chemin. Aucun
//! parallélisme. Écrit d'après la description publique de l'algorithme (E. W. Dijkstra,
//! « A note on two problems in connexion with graphs », 1959) ; aucune ligne de GRASS, de
//! GDAL, de QGIS ni de Whitebox n'est reprise.
//!
//! Mémoire : 12 octets par cellule (coût cumulé `f64`, prédécesseur `u32`), plus le tas. Le
//! tas garde les entrées périmées (suppression paresseuse) : une cellule peut y entrer jusqu'à
//! 8 fois, une par voisine, à 16 octets l'entrée, soit au pire 128 octets par cellule
//! franchissable. Ce module ne dépend pas du registre : il ne lit que des grilles `Dem` déjà
//! ouvertes par `dem.rs`.

use std::cmp::Ordering;
use std::collections::BinaryHeap;

use crate::dem::Dem;

/// Marque d'une cellule sans prédécesseur : source, ou cellule jamais atteinte.
pub const NO_PREDECESSOR: u32 = u32::MAX;

/// Coût cumulé de chaque cellule et prédécesseur sur le chemin de moindre coût.
#[derive(Debug, Clone, PartialEq)]
pub struct Accumulated {
    pub width: u32,
    pub height: u32,
    /// Coût cumulé depuis la source la plus proche, ligne par ligne ; `f64::INFINITY` pour une
    /// cellule infranchissable ou inatteignable.
    pub cost: Vec<f64>,
    /// Index de la cellule précédente sur le chemin, `NO_PREDECESSOR` pour une source ou une
    /// cellule jamais atteinte.
    pub predecessor: Vec<u32>,
}

impl Accumulated {
    /// Coût cumulé de la cellule (`col`, `row`), `None` si elle n'est pas atteinte.
    pub fn at(&self, col: u32, row: u32) -> Option<f64> {
        let v = self.cost[row as usize * self.width as usize + col as usize];
        v.is_finite().then_some(v)
    }
}

/// Coût par mètre de la cellule, `None` si elle est infranchissable.
pub fn passable(cost: &Dem, col: u32, row: u32) -> Option<f64> {
    cost.value(col, row)
        .map(f64::from)
        .filter(|v| v.is_finite() && *v > 0.0)
}

/// Entrée du tas : ordre inversé, pour que le tas max de la bibliothèque sorte le plus petit
/// coût, puis le plus petit index.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Entry {
    cost: f64,
    index: u32,
}

impl Eq for Entry {}

impl Ord for Entry {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .cost
            .total_cmp(&self.cost)
            .then(other.index.cmp(&self.index))
    }
}

impl PartialOrd for Entry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Coût cumulé depuis `sources` (colonne, ligne), toutes franchissables et dans la grille.
/// Une source infranchissable ou hors grille est ignorée : la commande la refuse avant. Une
/// source répétée ne change rien. Fonction pure et déterministe.
pub fn accumulate(cost: &Dem, sources: &[(u32, u32)]) -> Accumulated {
    let (w, h) = (cost.width as usize, cost.height as usize);
    let n = w * h;
    // `dem.rs` plafonne la grille à 1 Gio, soit 2^28 cellules : l'index tient en u32, et
    // `NO_PREDECESSOR` n'est jamais un index de cellule.
    assert!(
        n < NO_PREDECESSOR as usize,
        "raster de plus de 2^32 - 1 cellules"
    );
    let mut acc = vec![f64::INFINITY; n];
    let mut predecessor = vec![NO_PREDECESSOR; n];
    let mut heap = BinaryHeap::new();
    for &(col, row) in sources {
        if col as usize >= w || row as usize >= h || passable(cost, col, row).is_none() {
            continue;
        }
        let i = row as usize * w + col as usize;
        if acc[i] != 0.0 {
            acc[i] = 0.0;
            heap.push(Entry {
                cost: 0.0,
                index: i as u32,
            });
        }
    }
    let straight_x = cost.cell_size_x;
    let straight_y = cost.cell_size_y;
    let diagonal = cost.cell_size_x.hypot(cost.cell_size_y);
    while let Some(Entry { cost: d, index }) = heap.pop() {
        let i = index as usize;
        if d > acc[i] {
            continue;
        }
        let (col, row) = (i % w, i / w);
        // Toujours franchissable : seules des cellules franchissables entrent au tas.
        let here = passable(cost, col as u32, row as u32).unwrap_or(f64::INFINITY);
        for dr in -1i64..=1 {
            for dc in -1i64..=1 {
                if dr == 0 && dc == 0 {
                    continue;
                }
                let (c, r) = (col as i64 + dc, row as i64 + dr);
                if c < 0 || r < 0 || c >= w as i64 || r >= h as i64 {
                    continue;
                }
                let Some(there) = passable(cost, c as u32, r as u32) else {
                    continue;
                };
                // En biais, les deux cellules frôlées doivent être franchissables.
                if dc != 0
                    && dr != 0
                    && (passable(cost, c as u32, row as u32).is_none()
                        || passable(cost, col as u32, r as u32).is_none())
                {
                    continue;
                }
                let step = match (dc, dr) {
                    (0, _) => straight_y,
                    (_, 0) => straight_x,
                    _ => diagonal,
                };
                let next = d + (here + there) / 2.0 * step;
                let j = r as usize * w + c as usize;
                if next < acc[j] {
                    acc[j] = next;
                    predecessor[j] = index;
                    heap.push(Entry {
                        cost: next,
                        index: j as u32,
                    });
                }
            }
        }
    }
    Accumulated {
        width: cost.width,
        height: cost.height,
        cost: acc,
        predecessor,
    }
}

/// Cellules du chemin de moindre coût, de la source jusqu'à `end` (colonne, ligne), dans cet
/// ordre. Vide si `end` n'est pas atteinte ; une seule cellule si `end` est une source.
pub fn path(acc: &Accumulated, end: (u32, u32)) -> Vec<(u32, u32)> {
    let w = acc.width as usize;
    let mut i = end.1 as usize * w + end.0 as usize;
    if !acc.cost[i].is_finite() {
        return Vec::new();
    }
    let mut cells = vec![end];
    while acc.predecessor[i] != NO_PREDECESSOR {
        i = acc.predecessor[i] as usize;
        cells.push(((i % w) as u32, (i / w) as u32));
    }
    cells.reverse();
    cells
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Grille de 10 m, coin haut-gauche (0, h × 10), sans-donnée -9999.
    fn grid(width: u32, height: u32, values: Vec<f32>) -> Dem {
        Dem {
            width,
            height,
            origin_x: 0.0,
            origin_y: f64::from(height) * 10.0,
            cell_size_x: 10.0,
            cell_size_y: 10.0,
            epsg: 2154,
            nodata: Some(-9999.0),
            values,
        }
    }

    fn uniform(width: u32, height: u32) -> Dem {
        grid(width, height, vec![1.0; (width * height) as usize])
    }

    #[test]
    fn a_uniform_cost_along_a_row_is_the_distance() {
        let dem = uniform(9, 5);
        let acc = accumulate(&dem, &[(1, 2)]);
        assert_eq!(acc.at(1, 2), Some(0.0));
        assert_eq!(acc.at(7, 2), Some(60.0));
        let cells = path(&acc, (7, 2));
        assert_eq!(cells, (1..=7).map(|c| (c, 2)).collect::<Vec<_>>());
    }

    #[test]
    fn a_diagonal_step_costs_root_two_times_the_cell() {
        let dem = uniform(5, 5);
        let acc = accumulate(&dem, &[(0, 0)]);
        let expected = 4.0 * 10.0 * std::f64::consts::SQRT_2;
        assert!((acc.at(4, 4).unwrap() - expected).abs() < 1e-9);
        assert_eq!(
            path(&acc, (4, 4)),
            (0..=4).map(|k| (k, k)).collect::<Vec<_>>()
        );
        // Coût moyen des deux cellules : 1 puis 3, soit 2 × 10 m.
        let mut values = vec![1.0; 25];
        values[1] = 3.0;
        let acc = accumulate(&grid(5, 5, values), &[(0, 0)]);
        assert_eq!(acc.at(1, 0), Some(20.0));
    }

    #[test]
    fn a_wall_is_crossed_only_through_its_opening() {
        // Mur à la colonne 4, de NoData en haut et de 0 en bas, ouvert à la ligne 6.
        let (w, h) = (9u32, 8u32);
        let mut values = vec![1.0f32; (w * h) as usize];
        for row in 0..h {
            if row != 6 {
                values[(row * w + 4) as usize] = if row < 3 { -9999.0 } else { 0.0 };
            }
        }
        values[(2 * w + 6) as usize] = -3.0;
        let dem = grid(w, h, values);
        let acc = accumulate(&dem, &[(1, 1)]);
        let cells = path(&acc, (7, 1));
        assert_eq!(cells.first(), Some(&(1, 1)));
        assert_eq!(cells.last(), Some(&(7, 1)));
        assert!(cells.contains(&(4, 6)), "{cells:?}");
        for &(c, r) in &cells {
            assert!(passable(&dem, c, r).is_some(), "({c}, {r})");
        }
        // Le mur et la cellule négative ne sont jamais atteints.
        assert_eq!(acc.at(4, 0), None);
        assert_eq!(acc.at(4, 5), None);
        assert_eq!(acc.at(6, 2), None);
        // Le coût cumulé est celui du chemin, pas à pas.
        let mut total = 0.0;
        for pair in cells.windows(2) {
            let ((c0, r0), (c1, r1)) = (pair[0], pair[1]);
            let step = if c0 != c1 && r0 != r1 {
                200f64.sqrt()
            } else {
                10.0
            };
            total += step;
        }
        assert!((acc.at(7, 1).unwrap() - total).abs() < 1e-9);
    }

    #[test]
    fn several_sources_give_the_minimum_of_each_source_alone() {
        let values = (0..60).map(|i| 1.0 + (i % 7) as f32 * 0.5).collect();
        let dem = grid(10, 6, values);
        let (a, b) = ((0, 0), (8, 4));
        let both = accumulate(&dem, &[a, b]);
        let only_a = accumulate(&dem, &[a]);
        let only_b = accumulate(&dem, &[b]);
        for i in 0..both.cost.len() {
            assert_eq!(both.cost[i], only_a.cost[i].min(only_b.cost[i]), "{i}");
        }
        assert_eq!(both.at(0, 0), Some(0.0));
        assert_eq!(both.at(8, 4), Some(0.0));
        // Une source répétée ne change rien.
        assert_eq!(accumulate(&dem, &[a, b, a]), both);
        // Chaque chemin part d'une source.
        let start = path(&both, (9, 0))[0];
        assert!(start == a || start == b);
    }

    #[test]
    fn an_enclosed_zone_is_unreachable() {
        // Anneau de NoData autour des cellules (5..=7, 1..=3).
        let (w, h) = (10u32, 6u32);
        let mut values = vec![1.0f32; (w * h) as usize];
        for row in 0..=4 {
            for col in 4..=8 {
                if row == 0 || row == 4 || col == 4 || col == 8 {
                    values[(row * w + col) as usize] = -9999.0;
                }
            }
        }
        let dem = grid(w, h, values);
        let acc = accumulate(&dem, &[(0, 5)]);
        for row in 1..=3 {
            for col in 5..=7 {
                assert_eq!(acc.at(col, row), None, "({col}, {row})");
                assert!(passable(&dem, col, row).is_some());
            }
        }
        assert!(path(&acc, (6, 2)).is_empty());
        assert!(acc.at(9, 0).is_some());
        // Une source infranchissable est ignorée.
        assert!(accumulate(&dem, &[(4, 0)])
            .cost
            .iter()
            .all(|c| c.is_infinite()));
    }

    #[test]
    fn a_diagonal_wall_joined_by_its_corners_encloses_a_zone() {
        // Losange de NoData joint par ses coins autour de (3, 3) : seules des diagonales le
        // traverseraient, chacune entre deux cellules du mur.
        let (w, h) = (7u32, 7u32);
        let mut values = vec![1.0f32; (w * h) as usize];
        for (c, r) in [
            (3, 1),
            (4, 2),
            (5, 3),
            (4, 4),
            (3, 5),
            (2, 4),
            (1, 3),
            (2, 2),
        ] {
            values[(r * w + c) as usize] = -9999.0;
        }
        let dem = grid(w, h, values);
        let acc = accumulate(&dem, &[(0, 0)]);
        for (c, r) in [(3, 2), (2, 3), (3, 3), (4, 3), (3, 4)] {
            assert!(passable(&dem, c, r).is_some());
            assert_eq!(acc.at(c, r), None, "({c}, {r})");
        }
        assert!(path(&acc, (3, 3)).is_empty());
        assert!(acc.at(6, 6).is_some());
        // Un pas en biais à côté d'une seule cellule infranchissable est refusé aussi : le
        // chemin de (0, 0) à (1, 1) contourne le trou de (1, 0).
        let mut values = vec![1.0f32; 9];
        values[1] = -9999.0;
        let acc = accumulate(&grid(3, 3, values), &[(0, 0)]);
        assert_eq!(path(&acc, (1, 1)), vec![(0, 0), (0, 1), (1, 1)]);
        assert_eq!(acc.at(1, 1), Some(20.0));
    }

    #[test]
    fn a_source_alone_is_its_own_path() {
        let acc = accumulate(&uniform(3, 3), &[(1, 1)]);
        assert_eq!(path(&acc, (1, 1)), vec![(1, 1)]);
    }
}
