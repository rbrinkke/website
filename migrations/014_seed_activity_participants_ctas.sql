-- Seed CTA tiles for the participants strip on activity cards.
-- Idempotent: INSERT OR IGNORE by fixed UUIDs.

INSERT OR IGNORE INTO promotion_units (
  promotion_unit_id,
  placement,
  promo_group,
  locale,
  priority,
  weight,
  is_active,
  title,
  body,
  emoji,
  layout_kind,
  background_color,
  background_gradient,
  media_kind,
  media_asset_id,
  actions_json
) VALUES
-- JOIN CTAs
('9b1f0b7a-7f77-4c5e-94a2-5e4c33cbe0a1','activities_participants_cta','default','nl',0,3,1,NULL,'Doe mee!','⚡','participants_tile',NULL,'linear-gradient(135deg, #1E88E5, #FF0066)',NULL,NULL,
 json_array(json_object('kind','join','label','Aanmelden','icon','➕','method','POST'))),
('34f2a62d-3b41-4a34-8b4a-75c0c7a4a65c','activities_participants_cta','default','nl',0,2,1,NULL,'Join!','🚀','participants_tile',NULL,'linear-gradient(135deg, #1E88E5, rgba(255,0,102,0.85))',NULL,NULL,
 json_array(json_object('kind','join','label','Ga mee','icon','➕','method','POST'))),
('c49b5f1a-6b21-4c76-87de-3b69a4fcd9aa','activities_participants_cta','default','nl',0,2,1,NULL,'Zin om mee te gaan?','💙','participants_tile',NULL,'linear-gradient(135deg, rgba(11,18,32,0.92), #1E88E5)',NULL,NULL,
 json_array(json_object('kind','join','label','Doe mee','icon','➕','method','POST'))),
('0d77a9b9-f9aa-4c10-8c0d-4d2b11c2e7b6','activities_participants_cta','default','nl',0,1,1,NULL,'Laat je zien','✨','participants_tile',NULL,'linear-gradient(135deg, rgba(255,0,102,0.90), rgba(30,136,229,0.85))',NULL,NULL,
 json_array(json_object('kind','join','label','Aanmelden','icon','➕','method','POST'))),
('5f6c2c6f-0fdd-4f79-bb1f-8b5f3bb4d0a4','activities_participants_cta','default','nl',0,1,1,NULL,'Kom erbij','😊','participants_tile',NULL,'linear-gradient(135deg, rgba(30,136,229,0.92), rgba(11,18,32,0.90))',NULL,NULL,
 json_array(json_object('kind','join','label','Doe mee','icon','➕','method','POST'))),

-- WAITLIST CTAs
('e7d0a8d6-b6fe-4106-9b2d-86a6b2a6f9b2','activities_participants_cta','default','nl',0,3,1,NULL,'Join the list','📝','participants_tile',NULL,'linear-gradient(135deg, rgba(11,18,32,0.92), rgba(255,0,102,0.85))',NULL,NULL,
 json_array(json_object('kind','waitlist','label','Wachtlijst','icon','⏱','method','POST'))),
('2af3c06d-6794-4e30-8a6c-4b9a3c55f5b8','activities_participants_cta','default','nl',0,2,1,NULL,'Op de wachtlijst?','⏳','participants_tile',NULL,'linear-gradient(135deg, rgba(30,136,229,0.92), rgba(255,0,102,0.65))',NULL,NULL,
 json_array(json_object('kind','waitlist','label','Wachtlijst','icon','⏱','method','POST'))),
('1d4c52a0-0b8f-4e9f-b6f8-9a4e84b4e1d3','activities_participants_cta','default','nl',0,2,1,NULL,'Misschien komt er plek vrij','👀','participants_tile',NULL,'linear-gradient(135deg, rgba(255,0,102,0.88), rgba(11,18,32,0.92))',NULL,NULL,
 json_array(json_object('kind','waitlist','label','Wachtlijst','icon','⏱','method','POST'))),
('c1e6c977-59a8-4be1-9a6a-82c23e5f97c9','activities_participants_cta','default','nl',0,1,1,NULL,'Zet je op de lijst','✅','participants_tile',NULL,'linear-gradient(135deg, rgba(11,18,32,0.92), rgba(30,136,229,0.82))',NULL,NULL,
 json_array(json_object('kind','waitlist','label','Wachtlijst','icon','⏱','method','POST'))),
('7e8d4f9e-9f51-4d58-9c6e-4b5e7d2c3a11','activities_participants_cta','default','nl',0,1,1,NULL,'We houden je op de hoogte','📬','participants_tile',NULL,'linear-gradient(135deg, rgba(30,136,229,0.92), rgba(11,18,32,0.92))',NULL,NULL,
 json_array(json_object('kind','waitlist','label','Wachtlijst','icon','⏱','method','POST')));

