import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import 'app.dart';
import 'services/notification_service.dart';
import 'services/push_service.dart';
import 'state/agent_controller.dart';
import 'state/applications_controller.dart';
import 'state/auth_controller.dart';
import 'state/connection_controller.dart';
import 'state/jobs_controller.dart';
import 'state/nav_controller.dart';
import 'state/notifications_controller.dart';
import 'state/profiles_controller.dart';
import 'state/update_controller.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  // Firebase push is optional: without google-services.json in the build
  // this logs and the app relies on the WebSocket and periodic checks.
  try {
    await PushService.instance.init();
  } catch (e) {
    debugPrint('push setup failed: $e');
  }
  // The alerts channel must exist before any push arrives, and a
  // notification tap that launched the app should open its screen.
  try {
    await NotificationService.instance.start();
  } catch (e) {
    debugPrint('notification setup failed: $e');
  }
  final auth = AuthController();
  await auth.loadPersisted();

  runApp(
    MultiProvider(
      providers: [
        ChangeNotifierProvider.value(value: auth),
        ChangeNotifierProvider(create: (_) => NavController()),
        ChangeNotifierProvider(create: (_) => UpdateController(auth), lazy: false),
        ChangeNotifierProxyProvider<AuthController, ConnectionController>(
          create: (_) => ConnectionController(auth),
          update: (_, auth, previous) => previous ?? ConnectionController(auth),
        ),
        ChangeNotifierProxyProvider<ConnectionController, NotificationsController>(
          create: (context) => NotificationsController(auth, context.read<ConnectionController>()),
          update: (_, connection, previous) => previous ?? NotificationsController(auth, connection),
          lazy: false,
        ),
        ChangeNotifierProxyProvider<ConnectionController, ApplicationsController>(
          create: (context) => ApplicationsController(context.read<ConnectionController>()),
          update: (_, connection, previous) => previous ?? ApplicationsController(connection),
        ),
        ChangeNotifierProxyProvider<ConnectionController, JobsController>(
          create: (context) => JobsController(context.read<ConnectionController>()),
          update: (_, connection, previous) => previous ?? JobsController(connection),
        ),
        ChangeNotifierProxyProvider<ConnectionController, AgentController>(
          create: (context) => AgentController(context.read<ConnectionController>()),
          update: (_, connection, previous) => previous ?? AgentController(connection),
        ),
        ChangeNotifierProxyProvider<ConnectionController, ProfilesController>(
          create: (context) => ProfilesController(context.read<ConnectionController>()),
          update: (_, connection, previous) => previous ?? ProfilesController(connection),
        ),
        ChangeNotifierProxyProvider<ConnectionController, AnswersController>(
          create: (context) => AnswersController(context.read<ConnectionController>()),
          update: (_, connection, previous) => previous ?? AnswersController(connection),
        ),
      ],
      child: const JobHunterApp(),
    ),
  );
}
