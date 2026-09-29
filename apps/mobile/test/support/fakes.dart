import 'package:job_hunter_mobile/models/envelope.dart';
import 'package:job_hunter_mobile/state/connection_controller.dart';

/// A `ConnectionController` stand-in that records every `request()` call
/// and returns whatever the test stubs, so controllers can be exercised
/// without a real server or WebSocket. It never touches the network --
/// `ConnectionController` itself is a no-op here because the underlying
/// `AuthController` is never marked signed in -- and [serverGet] answers
/// from [serverData] (null, i.e. "unreachable", by default).
class FakeConnectionController extends ConnectionController {
  FakeConnectionController(super.auth);

  final List<({String type, Map<String, dynamic> payload})> calls = [];
  final Map<String, RelayResponse> stubbed = {};
  final Map<String, dynamic> serverData = {};
  final List<String> serverReads = [];

  @override
  Future<RelayResponse> request(String type, [Map<String, dynamic> payload = const {}]) async {
    calls.add((type: type, payload: payload));
    return stubbed[type] ?? const RelayResponse(ok: false, errorCode: 'NOT_STUBBED', errorMessage: 'This test did not stub a response for this request type.');
  }

  @override
  Future<dynamic> serverGet(String path) async {
    serverReads.add(path);
    return serverData[path];
  }
}
