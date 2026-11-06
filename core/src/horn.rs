//! Fenêtre 3 × 3 de Horn, commune à `terrain.slope`, `terrain.aspect` et `terrain.hillshade`.
//!
//! Provenance : les différences finies pondérées sur la fenêtre 3 × 3 sont celles de
//! B. K. P. Horn, « Hill shading and the reflectance map », *Proceedings of the IEEE*, 69(1),
//! 1981, p. 14-47, écrites d'après la publication seule. Aucune ligne de code de GDAL, de
//! GRASS, de QGIS ni de Whitebox n'est reprise.
//!
//! Ce module ne dépend pas du registre : il ne lit et n'écrit que des grilles `Dem` (AD-20).

use crate::dem::Dem;

/// Gradient de Horn de la cellule (`col`, `row`) : `(dx, dy)`, en mètres d'altitude par mètre.
///
/// `dx` est la dérivée vers l'est (positive quand le sol monte vers l'est) ; les lignes allant
/// du nord au sud, `dy` est la dérivée vers le sud (positive quand le sol monte vers le sud).
/// Rend `None` pour une cellule de bord, une cellule sans donnée, ou une cellule dont l'une des
/// 8 voisines est sans donnée : aucune altitude n'est inventée (comportement par défaut de GDAL
/// et de GRASS). Calcul en f64.
pub fn horn_gradient(dem: &Dem, col: u32, row: u32) -> Option<(f64, f64)> {
    if col == 0 || row == 0 || col + 1 >= dem.width || row + 1 >= dem.height {
        return None;
    }
    let z = |c: u32, r: u32| dem.value(c, r).map(f64::from);
    // La cellule centrale ne sert pas au calcul, mais une cellule sans donnée le reste.
    z(col, row)?;
    let (a, b, c) = (z(col - 1, row - 1)?, z(col, row - 1)?, z(col + 1, row - 1)?);
    let (d, f) = (z(col - 1, row)?, z(col + 1, row)?);
    let (g, h, i) = (z(col - 1, row + 1)?, z(col, row + 1)?, z(col + 1, row + 1)?);
    let dx = ((c + 2.0 * f + i) - (a + 2.0 * d + g)) / (8.0 * dem.cell_size_x);
    let dy = ((g + 2.0 * h + i) - (a + 2.0 * b + c)) / (8.0 * dem.cell_size_y);
    Some((dx, dy))
}

/// Grille de même emprise, même cellule et même EPSG que `dem`, portant `values` et la valeur
/// sans-donnée `nodata`. Champs copiés un à un : un `..dem.clone()` dupliquerait toute la
/// grille d'entrée.
pub fn same_grid_as(dem: &Dem, values: Vec<f32>, nodata: f32) -> Dem {
    Dem {
        width: dem.width,
        height: dem.height,
        origin_x: dem.origin_x,
        origin_y: dem.origin_y,
        cell_size_x: dem.cell_size_x,
        cell_size_y: dem.cell_size_y,
        epsg: dem.epsg,
        nodata: Some(f64::from(nodata)),
        values,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grid(width: u32, height: u32, values: Vec<f32>) -> Dem {
        Dem {
            width,
            height,
            origin_x: 0.0,
            origin_y: 100.0,
            cell_size_x: 10.0,
            cell_size_y: 10.0,
            epsg: 2154,
            nodata: Some(-9999.0),
            values,
        }
    }

    #[test]
    fn the_gradient_points_east_and_south_and_skips_edges_and_holes() {
        // Altitude = 2 × colonne + 3 × ligne : monte vers l'est et vers le sud.
        let values = (0..9).map(|i| (2 * (i % 3) + 3 * (i / 3)) as f32).collect();
        let dem = grid(3, 3, values);
        let (dx, dy) = horn_gradient(&dem, 1, 1).unwrap();
        assert!((dx - 0.2).abs() < 1e-12 && (dy - 0.3).abs() < 1e-12);
        assert!(horn_gradient(&dem, 0, 1).is_none());
        assert!(horn_gradient(&dem, 1, 2).is_none());
        let mut holed = dem.clone();
        holed.values[8] = -9999.0;
        assert!(horn_gradient(&holed, 1, 1).is_none());
        let same = same_grid_as(&dem, vec![0.0; 9], -9999.0);
        assert_eq!(same.extent(), dem.extent());
        assert_eq!(same.nodata, Some(-9999.0));
    }
}
