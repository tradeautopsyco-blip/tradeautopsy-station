/* PROTOTYPE data — extracted from design handoff METRICS */
const METRICS = {
  discipline:{ group:'behavior', name:'Discipline', unitLong:'% adherence', listNow:'57%',
    yMax:100, lo:40, hi:65, invert:false, trend:'Improving', up:true,
    ranges:{ D:{avg:'58',date:'Today · 30 Jun',bars:[88,84,72,55,40,38,42]},
             W:{avg:'57',date:'31 May – 6 Jun',bars:[34,52,22,78,91,88,31]},
             M:{avg:'60',date:'June 2026',bars:[44,61,38,72,80,66,29,55,63,48,70,84,59,41,33,68,77,52,61,88,45,57,72,64,36,69,81,58,47,62]},
             '6M':{avg:'64',date:'Jan – Jun',bars:[71,68,62,59,66,57]},
             Y:{avg:'63',date:'2026',bars:[55,61,68,71,68,62,59,66,57,64,70,66]} },
    daily:{today:'82',avg:'64',u:'%',lead:"You're holding your plan <b>better than you usually do</b> by this hour."},
    weekly:{avg:'57',u:'%',pct:57,bars:[34,52,22,78,91,88,31],lead:"Over the last 7 days, your plan adherence averaged <b>57%</b>."},
    yearly:{now:'64',prev:'58',u:'% adherence',nowPct:88,prevPct:80,lead:"You're <b>more disciplined this year</b> than last — +6 points of adherence."} },

  stop:{ group:'behavior', name:'Stop respect', unitLong:'% honoured', listNow:'34%',
    yMax:100, lo:40, hi:65, invert:false, trend:'Needs work', up:false,
    ranges:{ D:{avg:'31',date:'Today · 30 Jun',bars:[60,55,40,30,20,18,25]},
             W:{avg:'34',date:'31 May – 6 Jun',bars:[40,30,55,20,15,45,38]},
             M:{avg:'38',date:'June 2026',bars:[28,44,20,52,30,18,40,33,25,48,22,55,30,15,38,42,20,33,48,25,30,52,18,40,28,45,33,20,38,30]},
             '6M':{avg:'41',date:'Jan – Jun',bars:[48,44,40,38,42,34]},
             Y:{avg:'40',date:'2026',bars:[44,40,48,42,38,34,30,38,34,40,44,38]} },
    daily:{today:'25',avg:'38',u:'%',lead:"You've moved your stop <b>twice already today</b> — below your usual restraint."},
    weekly:{avg:'34',u:'%',pct:34,bars:[40,30,55,20,15,45,38],lead:"You honoured your planned stop on just <b>34%</b> of trades this week."},
    yearly:{now:'40',prev:'46',u:'% honoured',nowPct:74,prevPct:84,lead:"Your stop discipline has <b>slipped vs last year</b> — −6 points. Highest-leverage fix."} },

  exit:{ group:'behavior', name:'Exit discipline', unitLong:'% on-plan', listNow:'45%',
    yMax:100, lo:40, hi:65, invert:false, trend:'Stabilising', up:true,
    ranges:{ D:{avg:'48',date:'Today · 30 Jun',bars:[70,62,50,44,40,38,52]},
             W:{avg:'45',date:'31 May – 6 Jun',bars:[38,55,30,62,48,52,33]},
             M:{avg:'50',date:'June 2026',bars:[44,52,38,60,48,40,55,50,42,58,36,62,48,40,52,55,38,48,60,42,50,58,36,52,44,55,48,40,52,46]},
             '6M':{avg:'52',date:'Jan – Jun',bars:[55,52,50,48,53,45]},
             Y:{avg:'51',date:'2026',bars:[52,50,55,53,50,48,45,50,45,52,55,50]} },
    daily:{today:'62',avg:'48',u:'%',lead:"Your exits are <b>closer to plan</b> than usual so far today."},
    weekly:{avg:'45',u:'%',pct:45,bars:[38,55,30,62,48,52,33],lead:"You exited on-plan on <b>45%</b> of trades over the last 7 days."},
    yearly:{now:'51',prev:'47',u:'% on-plan',nowPct:84,prevPct:78,lead:"You're <b>exiting more cleanly this year</b> — +4 points on-plan."} },

  winrate:{ group:'perf', name:'Win rate', unitLong:'% wins', listNow:'41%',
    yMax:100, lo:40, hi:55, invert:false, trend:'Improving', up:true,
    ranges:{ D:{avg:'29',date:'Today · 30 Jun',bars:[100,100,0,0,0,0,0]},
             W:{avg:'41',date:'31 May – 6 Jun',bars:[33,50,20,67,60,57,28]},
             M:{avg:'44',date:'June 2026',bars:[40,55,33,60,50,45,38,52,48,42,58,50,40,36,44,55,48,42,60,38,50,52,36,48,40,55,50,42,48,46]},
             '6M':{avg:'46',date:'Jan – Jun',bars:[50,48,45,44,49,41]},
             Y:{avg:'45',date:'2026',bars:[48,45,50,49,46,44,41,46,42,48,50,46]} },
    daily:{today:'29',avg:'44',u:'%',lead:"Win rate is <b>below your average</b> for this hour — two losers after the open."},
    weekly:{avg:'41',u:'%',pct:41,bars:[33,50,20,67,60,57,28],lead:"You won <b>41%</b> of trades over the last 7 days."},
    yearly:{now:'45',prev:'42',u:'% wins',nowPct:82,prevPct:76,lead:"Your win rate is <b>up vs last year</b> — +3 points."} },

  hold:{ group:'perf', name:'Avg hold', unitLong:'min', listNow:'9m', countLabel:true,
    yMax:30, lo:8, hi:15, invert:false, trend:'Below baseline', up:false,
    ranges:{ D:{avg:'7',date:'Today · 30 Jun',bars:[22,19,12,8,5,4,6]},
             W:{avg:'9',date:'31 May – 6 Jun',bars:[6,11,4,16,13,12,7]},
             M:{avg:'11',date:'June 2026',bars:[8,13,6,17,12,9,15,11,8,16,5,18,10,7,13,15,6,11,17,8,12,16,5,13,9,15,11,7,13,10]},
             '6M':{avg:'13',date:'Jan – Jun',bars:[16,15,13,12,14,9]},
             Y:{avg:'12',date:'2026',bars:[15,13,16,14,13,12,9,13,11,14,16,13]} },
    daily:{today:'4',avg:'9',u:'m',lead:"Your hold time dropped to <b>4 min</b> after the open — baseline is 18 min."},
    weekly:{avg:'9',u:'m',pct:30,bars:[6,11,4,16,13,12,7],lead:"You held trades for an average of <b>9 minutes</b> this week."},
    yearly:{now:'12',prev:'15',u:'min · 2026',nowPct:80,prevPct:100,lead:"You're <b>holding shorter than last year</b> — −3 min on average."} },

  trades:{ group:'perf', name:'Trades / day', unitLong:'trades', listNow:'7', countLabel:true,
    yMax:12, lo:5, hi:8, invert:true, trend:'Overtrading', up:false,
    ranges:{ D:{avg:'7',date:'Today · 30 Jun',bars:[1,1,0,2,1,1,1]},
             W:{avg:'6',date:'31 May – 6 Jun',bars:[4,5,3,9,8,7,2]},
             M:{avg:'5',date:'June 2026',bars:[3,5,2,8,6,4,7,5,3,9,2,8,5,3,6,7,2,5,8,3,6,7,2,5,4,7,5,3,6,4]},
             '6M':{avg:'5',date:'Jan – Jun',bars:[4,5,5,6,5,6]},
             Y:{avg:'5',date:'2026',bars:[4,5,4,5,5,6,7,5,6,5,4,5]} },
    daily:{today:'7',avg:'5',u:'',lead:"You've taken <b>7 trades</b> today — 2 over your 5-trade limit."},
    weekly:{avg:'6',u:'',pct:50,bars:[4,5,3,9,8,7,2],lead:"You averaged <b>6 trades a day</b> this week, above your limit."},
    yearly:{now:'5',prev:'4',u:'trades/day',nowPct:84,prevPct:70,lead:"You're <b>trading more this year</b> — +1 a day vs last year."} },
};

const METRIC_ORDER = { behavior:['discipline','stop','exit'], perf:['winrate','hold','trades'] };
const XLAB = {
  D:['9','10','11','12','1','2','3'],
  W:['Sun','Mon','Tue','Wed','Thu','Fri','Sat'],
  M:['1','','','','8','','','','15','','','','22','','','','29',''],
  '6M':['Jan','Feb','Mar','Apr','May','Jun'],
  Y:['J','F','M','A','M','J','J','A','S','O','N','D']
};

function band(v, lo, hi, invert) {
  if (invert) return v <= lo ? 'g' : (v <= hi ? 'a' : 'r');
  return v >= hi ? 'g' : (v >= lo ? 'a' : 'r');
}
