-- Unify participants CTA tiles to a single brand color (blue) for both join + waitlist.

UPDATE promotion_units
SET background_gradient = NULL,
    background_color = '#1E88E5'
WHERE placement = 'activities_participants_cta';

