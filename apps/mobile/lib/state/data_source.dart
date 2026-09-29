/// Where the data on screen came from, shown as a small banner so the user
/// knows how fresh it is.
enum DataSource {
  /// Nothing loaded yet.
  none,

  /// Saved on this phone from an earlier session.
  cache,

  /// Live from the desktop.
  desktop,

  /// What the desktop last synced to the server (the desktop is offline).
  server,
}
