use cem_ml::svg_resource_acquisition::geometry::{Matrix, ViewBoxMapping};
#[test]
fn transform_order_rotation_and_skew_are_finite_and_bounded() {
    let m = Matrix::parse_svg("translate(10 20) scale(2 3)").unwrap();
    assert_eq!(m.map_point([1., 2.]).unwrap(), [12., 26.]);
    let r = Matrix::parse_svg("rotate(90 10 20)")
        .unwrap()
        .map_point([11., 20.])
        .unwrap();
    assert!((r[0] - 10.).abs() < 1e-10 && (r[1] - 21.).abs() < 1e-10);
    assert!(
        (Matrix::parse_svg("skewX(45)")
            .unwrap()
            .map_point([0., 2.])
            .unwrap()[0]
            - 2.)
            .abs()
            < 1e-10
    );
    for text in [
        "translate(1px)",
        "rotate(NaN)",
        "scale(1e999)",
        "matrix(1 0 0 1 0)",
        "scale(1,)",
        "scale(1) garbage",
        "translate(/*x*/1)",
        "skewY(90)",
    ] {
        assert!(Matrix::parse_svg(text).is_err(), "{text}");
    }
    assert!(Matrix::parse_svg(&"scale(1) ".repeat(65)).is_err());
}
#[test]
fn viewbox_meet_slice_none_alignment_and_degenerate_cases() {
    let meet = ViewBoxMapping::resolve([10., 20., 40., 20.], [0., 0., 100., 100.], "xMidYMid meet")
        .unwrap();
    assert_eq!(meet.matrix.map_point([10., 20.]).unwrap(), [0., 25.]);
    let slice =
        ViewBoxMapping::resolve([10., 20., 40., 20.], [0., 0., 100., 100.], "xMaxYMax slice")
            .unwrap();
    assert_eq!(slice.matrix.map_point([10., 20.]).unwrap(), [-100., 0.]);
    let none = ViewBoxMapping::resolve([10., 20., 40., 20.], [5., 7., 100., 100.], "none").unwrap();
    assert_eq!(none.matrix.map_point([50., 40.]).unwrap(), [105., 107.]);
    assert!(
        ViewBoxMapping::resolve([0., 0., 0., 1.], [0., 0., 10., 10.], "xMidYMid")
            .unwrap()
            .disabled
    );
    assert!(ViewBoxMapping::resolve([0., 0., -1., 1.], [0., 0., 10., 10.], "none").is_err());
    assert!(ViewBoxMapping::resolve([0., 0., 1., 1.], [0., 0., 10., 10.], "invalid").is_err());
}
#[test]
fn inverse_rejects_singular_and_round_trips_composed_points() {
    let m = Matrix::parse_svg("translate(5 -7) rotate(35) scale(2 4)").unwrap();
    let p = m
        .inverse()
        .unwrap()
        .map_point(m.map_point([10., 20.]).unwrap())
        .unwrap();
    assert!((p[0] - 10.).abs() < 1e-10 && (p[1] - 20.).abs() < 1e-10);
    assert!(Matrix::parse_svg("scale(0)").unwrap().inverse().is_err());
}
