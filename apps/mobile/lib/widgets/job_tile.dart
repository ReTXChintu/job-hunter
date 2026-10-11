import 'package:flutter/material.dart';

import '../theme/theme.dart';
import 'common.dart';
import 'status_badge.dart';

/// A job or application in a list: title, company, badges, match, and when
/// it last moved. Both the Jobs and Applications tabs use it so the two
/// read the same.
class JobTile extends StatelessWidget {
  final String title;
  final String subtitle;
  final String status;
  final int? score;
  final bool? relevant;
  final List<Widget> tags;
  final String? footnote;
  final VoidCallback onTap;

  const JobTile({
    super.key,
    required this.title,
    required this.subtitle,
    required this.status,
    this.score,
    this.relevant,
    this.tags = const [],
    this.footnote,
    required this.onTap,
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final accent = statusColor(context, status);
    return Padding(
      padding: const EdgeInsets.fromLTRB(16, 0, 16, 10),
      child: AppCard(
        onTap: onTap,
        padding: EdgeInsets.zero,
        child: IntrinsicHeight(
          child: Row(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              Container(width: 4, color: accent),
              Expanded(
                child: Padding(
                  padding: const EdgeInsets.fromLTRB(14, 12, 12, 12),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Row(crossAxisAlignment: CrossAxisAlignment.start, children: [
                        Expanded(
                          child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
                            Text(title.isEmpty ? 'Untitled job' : title, maxLines: 2, overflow: TextOverflow.ellipsis, style: theme.textTheme.titleSmall?.copyWith(fontWeight: FontWeight.w700, height: 1.2)),
                            const SizedBox(height: 3),
                            Text(subtitle, maxLines: 1, overflow: TextOverflow.ellipsis, style: theme.textTheme.bodySmall?.copyWith(color: theme.colorScheme.onSurfaceVariant)),
                          ]),
                        ),
                        const SizedBox(width: 10),
                        MatchRing(score: score, relevant: relevant),
                      ]),
                      const SizedBox(height: 10),
                      Wrap(spacing: 6, runSpacing: 4, crossAxisAlignment: WrapCrossAlignment.center, children: [StatusBadge(status: status), ...tags]),
                      if (footnote != null && footnote!.isNotEmpty) ...[
                        const SizedBox(height: 6),
                        Text(footnote!, style: theme.textTheme.bodySmall?.copyWith(color: theme.colorScheme.onSurfaceVariant, fontSize: 11.5)),
                      ],
                    ],
                  ),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

/// The match score as a small ring with the number inside.
class MatchRing extends StatelessWidget {
  final int? score;
  final bool? relevant;
  final double size;
  const MatchRing({super.key, this.score, this.relevant, this.size = 44});

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final color = matchColor(score, relevant);
    return SizedBox(
      width: size,
      height: size,
      child: Stack(
        alignment: Alignment.center,
        children: [
          CircularProgressIndicator(
            value: score == null ? 0 : score!.clamp(0, 100) / 100,
            strokeWidth: 4,
            color: color,
            backgroundColor: color.withValues(alpha: 0.15),
            strokeCap: StrokeCap.round,
          ),
          Text(
            score == null ? '–' : '$score',
            style: theme.textTheme.labelMedium?.copyWith(fontWeight: FontWeight.w800, color: score == null ? theme.colorScheme.onSurfaceVariant : color),
          ),
        ],
      ),
    );
  }
}
