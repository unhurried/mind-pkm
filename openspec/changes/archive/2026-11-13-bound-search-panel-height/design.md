## Decisions

Keep the current proportional panel sizing between bounds. Use the 22 logical pixel result row height plus 100 logical pixels for controls, headings, footer and spacing, giving bounds of 188 and 320 logical pixels.

Set the map window's minimum client height to 412 logical pixels: the minimum panel height, 64 pixels for bars and 160 pixels for the map. Apply the native minimum size only to the map window and raise remembered heights below this minimum before opening it.

## Validation

Measure the actual result scrollable after laying out the full map window at minimum and tall window sizes. Run the app with a disposable map and search results, inspect both bounds, and verify the native resize limit.
