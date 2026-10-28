# Design

The unsaved flag of the map is cleared only by saving or by replacing the map. Don't Save just goes on with what the question was asked for (exit, new, open); an exit may still be cancelled at the pages question, and opening may be cancelled in the file dialog, so the map must keep its flag until it is really gone.
