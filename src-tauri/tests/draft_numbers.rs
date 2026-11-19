//! Cantidades en cifras y en palabras (FR-016).

use app_lib::domain::draft::numbers::parse_quantity_text;
use app_lib::domain::draft::units::UnitCatalog;
use app_lib::domain::rate::UnitView;

fn quantity(text: &str) -> Option<i64> {
    parse_quantity_text(text, &UnitCatalog::default())
}

#[test]
fn digits_and_decimal_commas() {
    assert_eq!(quantity("3"), Some(300));
    assert_eq!(quantity("12"), Some(1_200));
    assert_eq!(quantity("1,5"), Some(150));
    assert_eq!(quantity("2.25"), Some(225));
    assert_eq!(quantity("3 metros"), Some(300));
}

#[test]
fn words_up_to_thousands() {
    let table = [
        ("uno", 1),
        ("dos", 2),
        ("tres", 3),
        ("diez", 10),
        ("quince", 15),
        ("dieciséis", 16),
        ("veinte", 20),
        ("veintidós", 22),
        ("treinta", 30),
        ("treinta y cinco", 35),
        ("noventa y nueve", 99),
        ("cien", 100),
        ("ciento veinte", 120),
        ("doscientos", 200),
        ("quinientos cuarenta y dos", 542),
        ("mil", 1_000),
        ("mil doscientos", 1_200),
        ("dos mil trescientos cincuenta", 2_350),
    ];
    for (text, expected) in table {
        assert_eq!(quantity(text), Some(expected * 100), "«{text}»");
    }
}

#[test]
fn usual_fractions() {
    let table = [
        ("metro y medio", 150),
        ("hora y media", 150),
        ("dos horas y media", 250),
        ("dos metros y medio", 250),
        ("dos y medio", 250),
        ("media hora", 50),
        ("medio metro", 50),
        ("cuarto de hora", 25),
        ("tres cuartos de hora", 75),
        ("un par de horas", 200),
        ("un par de", 200),
    ];
    for (text, expected) in table {
        assert_eq!(quantity(text), Some(expected), "«{text}»");
    }
}

#[test]
fn accents_and_capitals_do_not_matter() {
    assert_eq!(quantity("VEINTIDOS"), Some(2_200));
    assert_eq!(quantity("Metro y Medio"), Some(150));
}

#[test]
fn a_measure_of_the_material_is_not_a_quantity() {
    for text in [
        "20 mm",
        "20 milímetros",
        "veinte milímetros",
        "1,5 cm",
        "2 pulgadas",
        "treinta y cinco mm",
    ] {
        assert_eq!(quantity(text), None, "«{text}»");
    }
}

#[test]
fn the_quantity_after_a_measure_is_found() {
    assert_eq!(quantity("tubo 20 mm 3 metros"), Some(300));
    assert_eq!(quantity("tubo de 2 pulgadas, 4 unidades"), Some(400));
}

#[test]
fn a_unit_created_by_the_user_is_still_a_unit() {
    let units = UnitCatalog::new(&[UnitView {
        id: 9,
        label: "mm".into(),
        is_predefined: false,
    }]);
    assert_eq!(parse_quantity_text("20 mm", &units), Some(2_000));
}

#[test]
fn text_without_a_quantity() {
    assert_eq!(quantity("bajante"), None);
    assert_eq!(quantity(""), None);
    assert_eq!(quantity("cambiar la media pared"), None);
}
