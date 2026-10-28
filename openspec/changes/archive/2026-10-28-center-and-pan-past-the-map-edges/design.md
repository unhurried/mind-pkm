# Design

The map area is the laid-out map with a margin on each side: 40 px plus the room added on that side. The layout places the map after the margin on the left and top, so positions stay screen pixels in the scrollable's content.

Scrolling to an offset outside the map area (Esc's centering, a pan) first grows the room: by as much as the offset is below zero on the left and top (the map moves right and down by that much, and the offset with it, so the target becomes zero), and by as much as the view would reach past the far edges on the right and bottom. A pan in progress shifts its starting offset by the room added before the map, so the map keeps following the pointer. A message that placed the view this way doesn't also keep the focused node in place or reveal it afterwards.

When the map is scrolled (and when a pan ends), the room that is out of view is taken away: on the left and top as much as the offset (the offset shrinks by the same amount, so what is shown stays in place), on the right and bottom as much as lies beyond the view. The scroll position remembered for a map is the one without the room before it; opening a map starts without room.
