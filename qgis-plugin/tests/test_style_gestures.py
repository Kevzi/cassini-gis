"""Tests sans QGIS des gestes `layer.set_style` et `layer.arrange` (ticket 11.21).

Le fond QGIS est une doublure. Les cas de `fixtures/style-cases.json` sont lus aussi par le
noyau (`core/src/style/spec.rs`) : les deux côtés admettent et refusent les mêmes styles.
"""

import json
import os
import sys
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, "..", "cassini_qgis"))
STYLE_CASES = os.path.join(HERE, "..", "..", "fixtures", "style-cases.json")

import style_gestures  # noqa: E402
from gestures import GestureError  # noqa: E402


class FakeLayer:
    def __init__(self, layer_id, kind, geometry=None, fields=None):
        self.id, self.kind, self.geometry = layer_id, kind, geometry
        self.fields = dict(fields or {})
        self.applied = None
        self.visibility = "jamais posée"


class FakeGroup:
    def __init__(self, name, parent=None):
        self.name, self.children, self.parent = name, [], parent

    def add(self, *children):
        for child in children:
            if isinstance(child, FakeGroup):
                child.parent = self
            self.children.append(child)
        return self


class FakeBackend:
    """Un projet en mémoire : des couches, et un arbre de groupes et d'ids de couches.

    Comme QGIS, déplacer un groupe en rend une copie neuve et retire l'original : un appelant
    qui garderait l'identité Python d'un groupe déplacé se tromperait ici comme sous QGIS.
    """

    def __init__(self, layers=(), in_tree=None):
        self.layers = {layer.id: layer for layer in layers}
        self.root = FakeGroup(None)
        for layer in layers:
            if in_tree is None or layer.id in in_tree:
                self.root.children.append(layer.id)
        self.created = 0
        self.moves = 0
        self.no_classes = False

    # -- style
    def layer(self, layer_id):
        return self.layers.get(layer_id)

    def name_of(self, layer):
        return layer.id

    def kind_of(self, layer):
        return layer.kind

    def geometry_of(self, layer):
        return layer.geometry

    def field_kind(self, layer, name):
        return layer.fields.get(name)

    def apply_single(self, layer, geometry, symbol, opacity):
        layer.applied = ("single", geometry, symbol, opacity)

    def apply_categories(self, layer, geometry, field, classes, other, opacity):
        layer.applied = ("categories", geometry, field, classes, other)
        layer.opacity = opacity

    def apply_graduated(self, layer, geometry, field, method, class_count, ramp, symbol):
        if self.no_classes:  # comme le fond réel : rendu d'avant gardé, refus nommé
            raise GestureError("style.field_missing", "Aucune valeur à classer.")
        layer.applied = ("graduated", geometry, field, method, class_count, ramp, symbol)

    def apply_elevation_ramp(self, layer, stops, opacity):
        layer.applied = ("elevation_ramp", stops, opacity)

    def apply_hillshade(self, layer, opacity):
        layer.applied = ("hillshade", opacity)

    def apply_result_ramp(self, layer, stops, interpolation, transparent, unit, opacity):
        layer.applied = ("result_ramp", stops, interpolation, transparent, unit, opacity)

    def apply_visibility(self, layer, max_scale):
        layer.visibility = max_scale

    # -- arbre
    def _parent_of_layer(self, layer_id, group=None):
        group = group or self.root
        for child in group.children:
            if child == layer_id:
                return group
            if isinstance(child, FakeGroup):
                found = self._parent_of_layer(layer_id, child)
                if found is not None:
                    return found
        return None

    def has_tree_node(self, layer_id):
        return self._parent_of_layer(layer_id) is not None

    def root_groups(self):
        return [c for c in self.root.children if isinstance(c, FakeGroup)]

    def all_groups(self):
        found, todo = [], list(self.root.children)  # largeur d'abord, comme le fond réel
        while todo:
            node = todo.pop(0)
            if isinstance(node, FakeGroup):
                found.append(node)
                todo.extend(node.children)
        return found

    def group_name(self, group):
        return group.name

    def group_path(self, group):
        names = []
        while group is not None and group.parent is not None:
            names.append(group.name)
            group = group.parent
        return tuple(reversed(names))

    def layers_in(self, group):
        out = []
        for child in group.children:
            out.extend(self.layers_in(child) if isinstance(child, FakeGroup) else [child])
        return out

    def has_layers(self, group):
        return bool(self.layers_in(group))

    def has_children(self, group):
        return bool(group.children)

    def new_group(self, name):
        self.created += 1
        group = FakeGroup(name)
        self.root.add(group)
        return group

    @staticmethod
    def _clone(group):
        copy = FakeGroup(group.name)
        for child in group.children:
            copy.add(FakeBackend._clone(child) if isinstance(child, FakeGroup) else child)
        return copy

    def move_group_to_end(self, group):
        copy = self._clone(group)
        self.root.add(copy)
        group.parent.children.remove(group)
        group.parent, group.children, group.name = None, [], "<retiré>"  # l'original est mort
        return copy

    def move_layer_to_top(self, layer_id, group):
        target = group if group is not None else self.root
        self._parent_of_layer(layer_id).children.remove(layer_id)
        target.children.insert(0, layer_id)
        self.moves += 1

    def remove_group(self, group):
        group.parent.children.remove(group)

    def tree(self, group=None):
        """L'arbre en listes : un id, ou (nom, [enfants])."""
        group = group or self.root
        return [
            (c.name, self.tree(c)) if isinstance(c, FakeGroup) else c for c in group.children
        ]


def run(gesture, params):
    got = {}
    gesture.start(params, lambda result=None, error=None: got.update(result=result, error=error))
    return got.get("result"), got.get("error")


def set_style(backend, layer_id, style):
    gesture = style_gestures.SetStyleGesture(backend, schedule=lambda fn: fn())
    return run(gesture, {"layer_id": layer_id, "style": style})


def arrange(backend, groups):
    gesture = style_gestures.ArrangeGesture(backend, schedule=lambda fn: fn())
    return run(gesture, {"groups": groups})


def project(in_tree=None):
    return FakeBackend(
        [
            FakeLayer("plu", "vector", "polygon", {"typezone": "other", "surface": "numeric"}),
            FakeLayer("routes", "vector", "line", {"importance": "other"}),
            FakeLayer("gares", "vector", "point", {"nom": "other"}),
            FakeLayer("table", "vector", None, {"x": "numeric"}),
            FakeLayer("mnt", "raster"),
            FakeLayer("ombre", "raster"),
        ],
        in_tree,
    )


def without(backend, *ids):
    """Retire ces couches de la racine (pour les poser ailleurs dans l'arbre)."""
    backend.root.children = [c for c in backend.root.children if c not in ids]


class SharedStyleCasesTest(unittest.TestCase):
    """Les cas de `fixtures/style-cases.json`, que le noyau lit aussi."""

    def test_the_extension_accepts_and_refuses_the_shared_cases(self):
        with open(STYLE_CASES, encoding="utf-8") as f:
            cases = json.load(f)["cases"]
        self.assertGreaterEqual(len(cases), 30)
        for case in cases:
            with self.subTest(case["name"]):
                try:
                    style_gestures.normalize_style(case["style"])
                    got = None
                except GestureError as exc:
                    got = exc.code
                self.assertEqual(got, case["code"])


class SetStyleTest(unittest.TestCase):
    def test_single_symbol_takes_the_layer_geometry(self):
        backend = project()
        style = {
            "kind": "single",
            "stroke": "#AA0000",
            "fill": "#00ff00",
            "width_mm": 0.4,
            "opacity": 0.8,
        }
        symbol = {"stroke": "#aa0000", "fill": "#00ff00", "width_mm": 0.4}
        for layer_id, geometry in (("plu", "polygon"), ("routes", "line"), ("gares", "point")):
            result, error = set_style(backend, layer_id, style)
            self.assertIsNone(error)
            self.assertEqual(result, {"layer_id": layer_id})
            self.assertEqual(backend.layers[layer_id].applied, ("single", geometry, symbol, 0.8))

    def test_a_fill_only_surface_has_no_outline_and_full_opacity(self):
        backend = project()
        _, error = set_style(backend, "plu", {"kind": "single", "fill": "#ffffff"})
        self.assertIsNone(error)
        symbol = {"stroke": None, "fill": "#ffffff", "width_mm": None}
        self.assertEqual(backend.layers["plu"].applied, ("single", "polygon", symbol, 1.0))

    def test_categories_carry_each_class_and_the_other_symbol(self):
        backend = project()
        style = {
            "kind": "categories",
            "field": "typezone",
            "classes": [
                {"values": ["U", "Uh"], "label": "Urbaine", "stroke": "#000000",
                 "fill": "#e60000"},
                {"values": ["N"], "label": "Naturelle", "stroke": "#000000",
                 "fill": "#56aa02", "width_mm": 0.2},
            ],
            "other": {"label": "Autres", "stroke": "#808080"},
        }
        _, error = set_style(backend, "plu", style)
        self.assertIsNone(error)
        kind, geometry, field, classes, other = backend.layers["plu"].applied
        self.assertEqual((kind, geometry, field), ("categories", "polygon", "typezone"))
        self.assertEqual([c["values"] for c in classes], [["U", "Uh"], ["N"]])
        self.assertEqual(
            classes[1]["symbol"], {"stroke": "#000000", "fill": "#56aa02", "width_mm": 0.2}
        )
        other_symbol = {"stroke": "#808080", "fill": None, "width_mm": None}
        self.assertEqual(other, {"label": "Autres", "symbol": other_symbol, "max_scale": None})
        self.assertEqual([c["max_scale"] for c in classes], [None, None])
        self.assertEqual(backend.layers["plu"].opacity, 1.0)  # opaque par défaut

    def test_categories_carry_the_layer_opacity(self):
        backend = project()
        style = {
            "kind": "categories",
            "field": "typezone",
            "opacity": 0.6,
            "classes": [{"values": ["U"], "label": "U", "stroke": "#343434", "fill": "#e60000"}],
        }
        _, error = set_style(backend, "plu", style)
        self.assertIsNone(error)
        self.assertEqual(backend.layers["plu"].opacity, 0.6)

    def test_categories_hand_each_class_its_scale_threshold(self):
        backend = project()
        style = {
            "kind": "categories",
            "field": "importance",
            "classes": [
                {"values": ["1"], "label": "Importance 1", "stroke": "#000000"},
                {"values": ["5"], "label": "Importance 5", "stroke": "#000000",
                 "max_scale": 100000},
            ],
            "other": {"label": "Autre route", "stroke": "#808080", "max_scale": 100000},
            "max_scale": 500000,
        }
        _, error = set_style(backend, "routes", style)
        self.assertIsNone(error)
        _, _, _, classes, other = backend.layers["routes"].applied
        self.assertEqual([c["max_scale"] for c in classes], [None, 100000])
        self.assertEqual(other["max_scale"], 100000)
        self.assertEqual(backend.layers["routes"].visibility, 500000)  # la couche, en plus

    def test_categories_on_a_missing_field_are_refused(self):
        backend = project()
        style = {
            "kind": "categories",
            "field": "absent",
            "classes": [{"values": ["U"], "label": "U", "stroke": "#000000"}],
        }
        _, error = set_style(backend, "plu", style)
        self.assertEqual(error.code, "style.field_missing")
        self.assertIsNone(backend.layers["plu"].applied)

    def test_graduated_hands_the_method_and_the_ramp_to_qgis(self):
        backend = project()
        style = {
            "kind": "graduated",
            "field": "surface",
            "method": "jenks",
            "class_count": 5,
            "ramp": {"from": "#ffffb2", "to": "#BD0026"},
            "stroke": "#333333",
            "width_mm": 0.1,
        }
        _, error = set_style(backend, "plu", style)
        self.assertIsNone(error)
        symbol = {"stroke": "#333333", "fill": None, "width_mm": 0.1}
        self.assertEqual(
            backend.layers["plu"].applied,
            ("graduated", "polygon", "surface", "jenks", 5, ("#ffffb2", "#bd0026"), symbol),
        )

    def test_graduated_needs_an_existing_numeric_field(self):
        backend = project()
        base = {
            "kind": "graduated",
            "method": "quantile",
            "class_count": 4,
            "ramp": {"from": "#ffffff", "to": "#000000"},
        }
        for field in ("absent", "typezone"):
            _, error = set_style(backend, "plu", dict(base, field=field))
            self.assertEqual(error.code, "style.field_missing", field)
        self.assertIsNone(backend.layers["plu"].applied)

    def test_graduated_without_classes_keeps_the_old_style(self):
        backend = project()
        backend.no_classes = True
        style = {
            "kind": "graduated",
            "field": "surface",
            "method": "jenks",
            "class_count": 5,
            "ramp": {"from": "#ffffff", "to": "#000000"},
        }
        _, error = set_style(backend, "plu", style)
        self.assertEqual(error.code, "style.field_missing")
        self.assertIsNone(backend.layers["plu"].applied)

    def test_an_elevation_ramp_goes_to_a_raster(self):
        backend = project()
        stops = [
            {"value": 0, "color": "#00a000"},
            {"value": 500, "color": "#c8a050"},
            {"value": 2000, "color": "#ffffff"},
        ]
        style = {"kind": "elevation_ramp", "stops": stops, "opacity": 0.9}
        _, error = set_style(backend, "mnt", style)
        self.assertIsNone(error)
        expected = [(0.0, "#00a000"), (500.0, "#c8a050"), (2000.0, "#ffffff")]
        self.assertEqual(backend.layers["mnt"].applied, ("elevation_ramp", expected, 0.9))

    def test_a_hillshade_keeps_its_opacity(self):
        backend = project()
        _, error = set_style(backend, "ombre", {"kind": "hillshade", "opacity": 0.55})
        self.assertIsNone(error)
        self.assertEqual(backend.layers["ombre"].applied, ("hillshade", 0.55))

    def test_a_viewshed_ramp_hides_its_dictated_values(self):
        backend = project()
        style = {
            "kind": "result_ramp",
            "interpolation": "discrete",
            "stops": [{"value": 1, "color": "#B2182B", "label": "Visible"}],
            "transparent": [0, -9999],
            "opacity": 0.5,
        }
        _, error = set_style(backend, "mnt", style)
        self.assertIsNone(error)
        self.assertEqual(
            backend.layers["mnt"].applied,
            ("result_ramp", [(1.0, "#b2182b", "Visible")], "discrete", [0.0, -9999.0], None, 0.5),
        )

    def test_a_slope_ramp_is_interpolated_with_its_unit(self):
        backend = project()
        style = {
            "kind": "result_ramp",
            "interpolation": "linear",
            "unit": "°",
            "stops": [{"value": 0, "color": "#ffffcc"}, {"value": 42.5, "color": "#800026"}],
        }
        _, error = set_style(backend, "mnt", style)
        self.assertIsNone(error)
        self.assertEqual(
            backend.layers["mnt"].applied,
            (
                "result_ramp",
                [(0.0, "#ffffcc", None), (42.5, "#800026", None)],
                "linear",
                [],
                "°",
                1.0,
            ),
        )

    def test_legend_numbers_are_french_without_scientific_notation(self):
        label = style_gestures.number_label
        self.assertEqual(label(1234567.5), "1\u202f234\u202f567,5")
        self.assertEqual(label(88.91202545166016), "88,91")
        self.assertEqual(label(0.0), "0")
        self.assertEqual(label(-0.001), "0")
        self.assertEqual(label(-12.5), "-12,5")
        self.assertEqual(label(360), "360")

    def test_raster_styles_on_a_vector_are_a_geometry_mismatch(self):
        backend = project()
        stops = [{"value": 0, "color": "#000000"}, {"value": 1, "color": "#ffffff"}]
        ramp = {"kind": "result_ramp", "interpolation": "linear", "stops": stops}
        for style in ({"kind": "elevation_ramp", "stops": stops}, {"kind": "hillshade"}, ramp):
            _, error = set_style(backend, "plu", style)
            self.assertEqual(error.code, "style.geometry_mismatch")
        self.assertIsNone(backend.layers["plu"].applied)

    def test_vector_styles_on_a_raster_or_a_table_change_nothing(self):
        backend = project()
        for layer_id in ("mnt", "table"):
            _, error = set_style(backend, layer_id, {"kind": "single", "stroke": "#000000"})
            self.assertEqual(error.code, "style.geometry_mismatch", layer_id)
            self.assertIsNone(backend.layers[layer_id].applied)

    def test_an_unknown_layer_is_missing(self):
        backend = project()
        for layer_id in ("absente", ""):
            _, error = set_style(backend, layer_id, {"kind": "single", "stroke": "#000000"})
            self.assertEqual(error.code, "qgis.layer_missing")

    def test_a_malformed_style_changes_nothing(self):
        backend = project()
        for style in (None, {"kind": ["single"]}, {"kind": "single", "stroke": "red"}):
            _, error = set_style(backend, "plu", style)
            self.assertEqual(error.code, "style.invalid", style)
        self.assertTrue(all(layer.applied is None for layer in backend.layers.values()))

    def test_a_vector_style_carries_its_scale_visibility(self):
        backend = project()
        _, error = set_style(backend, "plu", {"kind": "single", "fill": "#ffffff",
                                              "max_scale": 50000})
        self.assertIsNone(error)
        self.assertEqual(backend.layers["plu"].visibility, 50000)
        # Sans `max_scale`, la couche redevient visible à toute échelle.
        set_style(backend, "plu", {"kind": "single", "fill": "#ffffff"})
        self.assertIsNone(backend.layers["plu"].visibility)

    def test_a_raster_style_leaves_the_scale_visibility_alone(self):
        backend = project()
        set_style(backend, "ombre", {"kind": "hillshade"})
        self.assertEqual(backend.layers["ombre"].visibility, "jamais posée")

    def test_a_refused_style_leaves_the_scale_visibility_alone(self):
        backend = project()
        _, error = set_style(backend, "plu", {"kind": "single", "fill": "#ffffff",
                                              "max_scale": 0})
        self.assertEqual(error.code, "style.invalid")
        self.assertEqual(backend.layers["plu"].visibility, "jamais posée")

    def test_an_unexpected_failure_is_named(self):
        backend = project()

        def boom(*args):
            raise RuntimeError("x")

        backend.apply_single = boom
        _, error = set_style(backend, "plu", {"kind": "single", "fill": "#ffffff"})
        self.assertEqual(error.code, "qgis.gesture_failed")


class ArrangeTest(unittest.TestCase):
    GROUPS = [
        {"name": "Transport", "layers": ["gares", "routes"]},
        {"name": "Urbanisme", "layers": ["plu"]},
        {"name": "Relief", "layers": ["ombre", "mnt"]},
    ]
    ARRANGED = [
        ("Transport", ["gares", "routes"]),
        ("Urbanisme", ["plu"]),
        ("Relief", ["ombre", "mnt"]),
    ]

    def test_groups_are_laid_in_order_under_the_unlisted_layers(self):
        backend = project()
        result, error = arrange(backend, self.GROUPS)
        self.assertIsNone(error)
        self.assertEqual(result, {"group_count": 3, "layer_count": 5})
        self.assertEqual(backend.tree(), ["table"] + self.ARRANGED)

    def test_a_second_arrange_reuses_the_groups_and_drops_an_emptied_one(self):
        backend = project()
        arrange(backend, self.GROUPS)
        again = [
            {"name": "Transport", "layers": ["routes", "gares", "plu"]},
            {"name": "Relief", "layers": ["mnt", "ombre"]},
        ]
        result, error = arrange(backend, again)
        self.assertIsNone(error)
        self.assertEqual(result, {"group_count": 2, "layer_count": 5})
        self.assertEqual(backend.created, 3)  # aucun groupe neuf au second passage
        self.assertEqual(
            backend.tree(),
            ["table", ("Transport", ["routes", "gares", "plu"]), ("Relief", ["mnt", "ombre"])],
        )

    def test_an_already_empty_user_group_is_left_alone(self):
        backend = project()
        backend.root.children.insert(0, FakeGroup("Mes brouillons", backend.root))
        arrange(backend, self.GROUPS)
        self.assertEqual(backend.tree()[0], ("Mes brouillons", []))

    def test_unlisted_layers_of_a_reused_group_go_back_above_the_groups(self):
        backend = project()
        without(backend, "table", "mnt")
        backend.root.add(FakeGroup("Relief").add("table", "mnt"))
        _, error = arrange(backend, self.GROUPS)
        self.assertIsNone(error)
        self.assertEqual(backend.tree(), ["table"] + self.ARRANGED)

    def test_an_emptied_sub_group_of_a_reused_group_is_removed(self):
        # Relief (repris) > Ancien (vidé par le rangement) et Brouillons (déjà vide, gardé).
        backend = project()
        without(backend, "mnt", "ombre")
        relief = FakeGroup("Relief").add(
            FakeGroup("Ancien").add("mnt", "ombre"), FakeGroup("Brouillons")
        )
        backend.root.add(relief)
        _, error = arrange(backend, self.GROUPS)
        self.assertIsNone(error)
        self.assertEqual(backend.tree()[-1], ("Relief", ["ombre", "mnt", ("Brouillons", [])]))
        self.assertEqual(backend.created, 2)  # Relief repris, pas recréé

    def test_a_nested_user_group_emptied_elsewhere_goes_and_an_empty_one_stays(self):
        # Travail > Couches (vidé : plu part à Urbanisme) et Vide (déjà vide) : Couches part,
        # Vide reste, donc Travail reste aussi.
        backend = project()
        without(backend, "plu")
        work = FakeGroup("Travail", backend.root).add(FakeGroup("Couches").add("plu"),
                                                      FakeGroup("Vide"))
        backend.root.children.insert(0, work)
        _, error = arrange(backend, self.GROUPS)
        self.assertIsNone(error)
        self.assertEqual(backend.tree()[0], ("Travail", [("Vide", [])]))

    def test_a_group_emptied_to_the_bone_goes_with_its_emptied_children(self):
        backend = project()
        without(backend, "plu")
        work = FakeGroup("Travail", backend.root).add(FakeGroup("Couches").add("plu"))
        backend.root.children.insert(0, work)
        arrange(backend, self.GROUPS)
        self.assertEqual(backend.tree(), ["table"] + self.ARRANGED)

    def test_an_empty_group_or_a_layer_cited_twice_is_refused(self):
        bad = [
            [],
            [{"name": "Vide", "layers": []}],
            [{"name": "A", "layers": ["plu"]}, {"name": "B", "layers": ["plu"]}],
            [{"name": "A", "layers": ["plu"]}, {"name": "A", "layers": ["mnt"]}],
            [{"name": "", "layers": ["plu"]}],
        ]
        for groups in bad:
            backend = project()
            _, error = arrange(backend, groups)
            self.assertEqual(error.code, "arrange.invalid", groups)
            self.assertEqual(backend.created, 0)

    def test_an_unknown_layer_is_missing_and_nothing_moves(self):
        backend = project()
        before = backend.tree()
        groups = [{"name": "A", "layers": ["plu"]}, {"name": "B", "layers": ["absente"]}]
        _, error = arrange(backend, groups)
        self.assertEqual(error.code, "qgis.layer_missing")
        self.assertEqual(backend.tree(), before)

    def test_a_layer_outside_the_tree_is_missing_and_nothing_moves(self):
        backend = project(in_tree={"plu", "routes", "gares", "table", "ombre"})  # mnt hors arbre
        before = backend.tree()
        _, error = arrange(backend, self.GROUPS)
        self.assertEqual(error.code, "qgis.layer_missing")
        self.assertIn("mnt", error.message)
        self.assertEqual(backend.tree(), before)
        self.assertEqual((backend.created, backend.moves), (0, 0))


if __name__ == "__main__":
    unittest.main()
