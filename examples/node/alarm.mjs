import { createTimebridge } from '../../extensions/sdk/dist/index.js';
const clock=createTimebridge({appName:'Example Node app',origin:'https://example.com',transport:'loopback',credential:process.env.TIMEBRIDGE_CREDENTIAL});
try {
  if(!(await clock.connect()).desktop)throw new Error('Open Timebridge Desktop first.');
  console.log('Approve Example Node app in Timebridge Desktop if requested.');
  await clock.requestPermission();
  const item=await clock.createCountdown({durationSeconds:10});
  console.log(`Created ${item.title}. ID: ${item.id}. The timer continues after this script exits.`);
} finally { clock.dispose(); }
