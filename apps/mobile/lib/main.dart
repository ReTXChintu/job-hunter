import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import 'app.dart';
import 'state/applications_controller.dart';
import 'state/auth_controller.dart';
import 'state/connection_controller.dart';
import 'state/update_controller.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  final auth = AuthController();
  await auth.loadPersisted();

  runApp(
    MultiProvider(
      providers: [
        ChangeNotifierProvider.value(value: auth),
        ChangeNotifierProvider(create: (_) => UpdateController(auth), lazy: false),
        ChangeNotifierProxyProvider<AuthController, ConnectionController>(
          create: (_) => ConnectionController(auth),
          update: (_, auth, previous) => previous ?? ConnectionController(auth),
        ),
        ChangeNotifierProxyProvider<ConnectionController, ApplicationsController>(
          create: (context) => ApplicationsController(context.read<ConnectionController>()),
          update: (_, connection, previous) => previous ?? ApplicationsController(connection),
        ),
      ],
      child: const JobHunterApp(),
    ),
  );
}
