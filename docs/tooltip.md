# Tooltips

`element::Tooltip` wraps any retained child and shows the host egui tooltip while
the pointer hovers over it. The transparent hover target fills the child's
layout box and does not claim clicks or gestures:

```rust
use rust_flutter::element::{Tooltip, Widget};

let save_button = Widget::empty(); // Replace with any retained control.
let described = Tooltip::new(save_button, "Save changes")
    .into_widget();
# let _ = described;
```

Use `.enabled(false)` to suppress the tooltip without changing the child. This
first version supports pointer hover only. Long-press display on touch, keyboard
focus behavior, placement and delay configuration, and screen-reader hint
semantics remain future work.
