pub fn html_preamble(asset_url_prefix: Option<&str>) -> String {
    let mut preamble = String::new();
    if let Some(prefix) = asset_url_prefix {
        let escaped = prefix.replace('\\', "\\\\").replace('"', "\\\"");
        preamble.push_str(&format!(
            r#"#show image: it => context {{
  if target() == "html" and type(it.source) == str {{
    let path = it.source.trim("./", at: start, repeat: false)
    let attrs = (src: "{escaped}" + path)
    if it.alt != none and type(it.alt) == str {{ attrs.insert("alt", it.alt) }}
    if type(it.width) == ratio {{
      attrs.insert("style", "width: " + str(float(it.width) * 100) + "%")
    }}
    html.elem("img", attrs: attrs)
  }} else {{ it }}
}}
"#
        ));
    }
    preamble.push_str(
        r#"#show align: it => context {
  if target() == "html" {
    let side = if it.alignment == center { "center" } else if it.alignment == right { "right" } else if it.alignment == left { "left" } else { none }
    if side != none { html.elem("div", attrs: (style: "text-align: " + side), it.body) } else { it.body }
  } else { it }
}
#show pad: it => context { if target() == "html" { it.body } else { it } }
"#,
    );
    preamble
}
