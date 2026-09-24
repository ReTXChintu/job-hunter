import 'package:flutter/material.dart';

class MatchScoreBar extends StatelessWidget {
  final int? score;
  final bool? relevant;
  const MatchScoreBar({super.key, this.score, this.relevant});

  @override
  Widget build(BuildContext context) {
    if (score == null) {
      return Text('Not analyzed', style: Theme.of(context).textTheme.bodySmall?.copyWith(color: Theme.of(context).colorScheme.onSurfaceVariant));
    }
    final color = relevant == false ? Colors.grey : (score! >= 85 ? Colors.green : (score! >= 60 ? Colors.blue : Colors.orange));
    return Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        SizedBox(
          width: 56,
          height: 6,
          child: ClipRRect(
            borderRadius: BorderRadius.circular(3),
            child: LinearProgressIndicator(value: score!.clamp(0, 100) / 100, color: color, backgroundColor: color.withValues(alpha: 0.15)),
          ),
        ),
        const SizedBox(width: 6),
        Text('$score%', style: TextStyle(fontWeight: FontWeight.w600, color: relevant == false ? Theme.of(context).colorScheme.onSurfaceVariant : null)),
      ],
    );
  }
}
