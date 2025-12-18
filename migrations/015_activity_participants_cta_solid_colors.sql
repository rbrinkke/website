-- Switch participants CTA tiles to solid brand colors (no gradients).

UPDATE promotion_units
SET background_gradient = NULL,
    background_color = CASE
      WHEN json_extract(actions_json, '$[0].kind') = 'waitlist' THEN '#0B1220'
      ELSE '#1E88E5'
    END
WHERE placement = 'activities_participants_cta';
