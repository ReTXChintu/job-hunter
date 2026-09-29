import 'dart:async';

import '../logic/links.dart';

/// Hands "open this screen" requests from notification taps (local
/// notifications, Firebase pushes, the app launched from a notification) to
/// the signed-in shell. A link that arrives before the shell exists (cold
/// start, or while signed out) waits in [pending] until it's taken.
class LinkBus {
  LinkBus._();
  static final instance = LinkBus._();

  final _controller = StreamController<AppLink>.broadcast();
  AppLink? _pending;

  Stream<AppLink> get links => _controller.stream;

  void open(AppLink? link) {
    if (link == null) return;
    _pending = link;
    _controller.add(link);
  }

  /// The waiting link, once: whoever takes it handles it.
  AppLink? take() {
    final link = _pending;
    _pending = null;
    return link;
  }
}
