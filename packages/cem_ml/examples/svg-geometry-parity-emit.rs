//! Explicit fixture source + scalar JSON export; never runtime AST/evidence ingress.
use cem_ml::{
    scheduler::AbortSignal,
    svg_computed_execution::*,
    svg_paint_clone_profile::SvgPaintCloneProfile,
    svg_resource_acquisition::{geometry::*, *},
};
use serde_json::json;
use std::sync::Arc;
#[path = "../tests/support/stylesheet_fixture.rs"]
mod fixture;
const URL: &str = "https://page.test/paint.svg";
struct Observer;
impl SvgComputedObserver for Observer {
    fn environment_revision(&mut self) -> Result<String, String> {
        Ok("fixture".into())
    }
    fn observe(&mut self, _: SvgComputedSlot<'_>) -> Result<String, String> {
        Ok(format!("url({URL}#p)"))
    }
}
fn record(content: &str) -> SvgResourceRecord {
    let (_, inv, cap) = fixture::setup(
        "<svg xmlns='http://www.w3.org/2000/svg'><rect fill='url(paint.svg#p)'/></svg>",
    );
    let sources: Vec<_> = inv
        .svg_presentations()
        .iter()
        .map(|p| p.source.clone())
        .collect();
    let mut host = SvgComputedHost::default();
    let mut observer = Observer;
    let e = Arc::new(host.capture(&sources, &mut observer).unwrap());
    let bytes = format!("<svg xmlns='http://www.w3.org/2000/svg'>{content}</svg>").into_bytes();
    let mut r = SvgResourceRecord::capture(
        e,
        &mut host,
        &mut observer,
        vec![SvgResourceResponse {
            requested_url: URL.into(),
            final_url: URL.into(),
            revision: "fixture".into(),
            content_type: "image/svg+xml".into(),
            bytes,
        }],
        AbortSignal::new(),
    )
    .unwrap();
    r.prepare_paint_content(
        &mut host,
        &mut observer,
        SvgPaintCloneProfile::for_browser("chromium", "148.0.7778.96").unwrap(),
        &cap,
        100000,
    )
    .unwrap();
    r
}
fn main() {
    let transforms:Vec<_>=["translate(10 20) scale(2 3)","rotate(90 10 20)","skewX(30) skewY(-15)","matrix(1 2 3 4 5 6) translate(-2, 3)","scale(0)"]
        .into_iter().map(|source|json!({"source":source,"matrix":Matrix::parse_svg(source).unwrap().coefficients()})).collect();
    let viewboxes:Vec<_>=["xMidYMid meet","xMaxYMax slice","none","xMinYMax meet"].into_iter().map(|aspect|{
        let vb=[10.,20.,40.,20.];let vp=[5.,7.,100.,100.];
        json!({"aspect":aspect,"viewBox":vb,"viewport":vp,"matrix":ViewBoxMapping::resolve(vb,vp,aspect).unwrap().matrix.coefficients()})
    }).collect();
    let gradients:Vec<_>=["", "gradientUnits='userSpaceOnUse' x1='0%' x2='100%'", "gradientTransform='rotate(90 .5 .5)'", "gradientTransform='translate(.25 0) scale(.5 1)'"].into_iter().map(|attrs|{
        let content=format!("<linearGradient id='p' {attrs}><stop offset='.5' stop-color='red'/><stop offset='.5' stop-color='blue'/></linearGradient>");
        let r=record(&content);let p=r.paint_content(0).unwrap().1;
        let g=p.resolve_gradient(GeometryContext{viewport:[100.,100.],bounding_box:[10.,20.,80.,40.]}).unwrap();
        let ResolvedGradient::Linear{start,end}=g.shape else{unreachable!()};
        let points:Vec<_>=[[20.,30.],[80.,50.]].into_iter().map(|point|{
            let local=g.paint_to_user.inverse().unwrap().map_point([point[0]+0.5,point[1]+0.5]).unwrap();
            let d=[end[0]-start[0],end[1]-start[1]];
            let t=((local[0]-start[0])*d[0]+(local[1]-start[1])*d[1])/(d[0]*d[0]+d[1]*d[1]);
            json!({"point":point,"pixel":if t<0.5{[255,0,0,255]}else{[0,0,255,255]}})
        }).collect();
        json!({"content":content,"points":points})
    }).collect();
    let composites:Vec<_>=[
        "<g opacity='.5'><rect width='16' height='16' fill='red'/><rect width='16' height='16' fill='blue'/></g>",
        "<rect width='16' height='16' fill='red' opacity='.5'/><rect width='16' height='16' fill='blue' opacity='.5'/>",
        "<g opacity='.5'><rect width='16' height='16' fill='red'/><g opacity='.5'><rect width='16' height='16' fill='blue'/></g></g>",
    ].into_iter().map(|children|{
        let content=format!("<pattern id='p' patternUnits='userSpaceOnUse' width='16' height='16'>{children}</pattern>");
        let r=record(&content);let p=r.paint_content(0).unwrap().1;
        let coverage:Vec<_>=p.colors.as_ref().unwrap().iter().map(|c|c.source).collect();
        json!({"content":content,"premultiplied":p.composite_pattern_sample(&coverage).unwrap().premultiplied_rgba})
    }).collect();
    println!(
        "{}",
        json!({"transforms":transforms,"viewboxes":viewboxes,"gradients":gradients,"composites":composites,"executionEvidence":false})
    );
}
