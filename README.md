# History

Bibliothek für Undo und Redo Operationen.
Konzipiert um mit verschiedenen Daten Snapshots eine chronologische Historie aufzubauen.

Wichtige Features:
- Ringpuffer Stack für endlosen Undo Spaß^^
- Multiple Daten Modelle in einem Stack möglich
- Lazy Baseline Initialisierung
- State Provider für Snapshot Restore-Operationen

Aktuelle Probleme:
- Typsicherheit der verschienden Modelle (Any ist doof)