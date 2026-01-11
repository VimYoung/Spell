//! Cursor Test Example
//!
//! This example creates a window with rectangles for each different mouse cursor type.
//! Hovering over each rectangle should change the cursor to the corresponding shape,
//! allowing visual verification that the Wayland cursor implementation works correctly.
//!
//! Run with: cargo run --example cursor_test

use std::error::Error;

use spell_framework::{
    cast_spell,
    layer_properties::{BoardType, LayerAnchor, LayerType, WindowConf},
    wayland_adapter::SpellWin,
};

slint::slint! {
    import { VerticalBox, HorizontalBox, GridBox, ScrollView } from "std-widgets.slint";

    component CursorTile inherits Rectangle {
        in property <string> label;
        in property <MouseCursor> cursor-type;

        width: 120px;
        height: 60px;
        border-radius: 8px;
        background: ta.has-hover ? #4a90d9 : #2d4a6a;
        border-width: 2px;
        border-color: ta.has-hover ? #7ab8f0 : #3d5a7a;

        ta := TouchArea {
            mouse-cursor: root.cursor-type;
        }

        Text {
            text: root.label;
            color: #ffffff;
            font-size: 11px;
            horizontal-alignment: center;
            vertical-alignment: center;
        }
    }

    export component CursorTest inherits Window {
        title: "Cursor Test - Hover to Test Each Cursor";
        background: #1a2a3a;

        ScrollView {
            horizontal-scrollbar-policy: always-off;

            VerticalBox {
                padding: 20px;
                spacing: 10px;

                Text {
                    text: "Cursor Test - Hover over each tile to test cursor shapes";
                    color: #ffffff;
                    font-size: 16px;
                    horizontal-alignment: center;
                }

                // Row 1: Basic cursors
                HorizontalBox {
                    spacing: 8px;
                    alignment: center;
                    CursorTile { label: "Default"; cursor-type: default; }
                    CursorTile { label: "None"; cursor-type: none; }
                    CursorTile { label: "Help"; cursor-type: help; }
                    CursorTile { label: "Pointer"; cursor-type: pointer; }
                }

                // Row 2: Progress/Wait
                HorizontalBox {
                    spacing: 8px;
                    alignment: center;
                    CursorTile { label: "Progress"; cursor-type: progress; }
                    CursorTile { label: "Wait"; cursor-type: wait; }
                    CursorTile { label: "Crosshair"; cursor-type: crosshair; }
                    CursorTile { label: "Text"; cursor-type: text; }
                }

                // Row 3: Drag/Drop
                HorizontalBox {
                    spacing: 8px;
                    alignment: center;
                    CursorTile { label: "Alias"; cursor-type: alias; }
                    CursorTile { label: "Copy"; cursor-type: copy; }
                    CursorTile { label: "Move"; cursor-type: move; }
                    CursorTile { label: "No-Drop"; cursor-type: no-drop; }
                }

                // Row 4: Grab/Forbidden
                HorizontalBox {
                    spacing: 8px;
                    alignment: center;
                    CursorTile { label: "Not-Allowed"; cursor-type: not-allowed; }
                    CursorTile { label: "Grab"; cursor-type: grab; }
                    CursorTile { label: "Grabbing"; cursor-type: grabbing; }
                    CursorTile { label: "Col-Resize"; cursor-type: col-resize; }
                }

                // Row 5: Resize directions
                HorizontalBox {
                    spacing: 8px;
                    alignment: center;
                    CursorTile { label: "Row-Resize"; cursor-type: row-resize; }
                    CursorTile { label: "N-Resize"; cursor-type: n-resize; }
                    CursorTile { label: "E-Resize"; cursor-type: e-resize; }
                    CursorTile { label: "S-Resize"; cursor-type: s-resize; }
                }

                // Row 6: More resize directions
                HorizontalBox {
                    spacing: 8px;
                    alignment: center;
                    CursorTile { label: "W-Resize"; cursor-type: w-resize; }
                    CursorTile { label: "NE-Resize"; cursor-type: ne-resize; }
                    CursorTile { label: "NW-Resize"; cursor-type: nw-resize; }
                    CursorTile { label: "SE-Resize"; cursor-type: se-resize; }
                }

                // Row 7: Bidirectional resize
                HorizontalBox {
                    spacing: 8px;
                    alignment: center;
                    CursorTile { label: "SW-Resize"; cursor-type: sw-resize; }
                    CursorTile { label: "EW-Resize"; cursor-type: ew-resize; }
                    CursorTile { label: "NS-Resize"; cursor-type: ns-resize; }
                    CursorTile { label: "NESW-Resize"; cursor-type: nesw-resize; }
                }

                HorizontalBox {
                    spacing: 8px;
                    alignment: center;
                    CursorTile { label: "NWSE-Resize"; cursor-type: nwse-resize; }
                }
            }
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let window_conf = WindowConf::new(
        560,                            // width
        520,                            // height
        (Some(LayerAnchor::TOP), None), // centered at top
        (50, 0, 0, 0),                  // margins
        LayerType::Top,
        BoardType::None,
        None,
        None,
    );

    let waywindow = SpellWin::invoke_spell("cursor-test", window_conf);
    let _ui = CursorTest::new().unwrap();

    println!("Cursor Test Started!");
    println!("Hover over each tile to test the Wayland cursor implementation.");
    println!("Press ESC to exit when the window has keyboard focus.");

    cast_spell(waywindow, None, None)
}
