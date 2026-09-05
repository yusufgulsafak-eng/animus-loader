(() => {
  const csrf = document.body.dataset.csrf;
  const panels = [...document.querySelectorAll('.panel')];
  const nav = [...document.querySelectorAll('[data-panel]')];
  const title = document.querySelector('#panel-title');
  const toast = (message, bad = false) => {
    const el = document.querySelector('#toast');
    el.textContent = message;
    el.classList.toggle('danger-text', bad);
    el.classList.add('show');
    setTimeout(() => el.classList.remove('show'), 3500);
  };
  const open = id => {
    panels.forEach(p => p.classList.toggle('active', p.id === 'panel-' + id));
    nav.forEach(b => b.classList.toggle('active', b.dataset.panel === id));
    const link = document.querySelector('[data-panel="' + id + '"]');
    const heading = document.querySelector('#panel-' + id + ' h2');
    title.textContent = link?.textContent || heading?.textContent || 'Yönetim';
    location.hash = id;
  };
  nav.forEach(b => b.addEventListener('click', () => open(b.dataset.panel)));
  document.querySelectorAll('[data-open]').forEach(b => b.addEventListener('click', () => open(b.dataset.open)));
  open(location.hash.slice(1) || 'dashboard');

  const api = async body => {
    const response = await fetch('/admin/action', {
      method: 'POST',
      headers: {'Content-Type':'application/json','X-CSRF-Token':csrf},
      body: JSON.stringify(body)
    });
    const json = await response.json();
    if (!response.ok || !json.ok) throw new Error(json.error?.message || 'İşlem başarısız');
    return json.data;
  };

  document.querySelector('#game-search')?.addEventListener('input', e => {
    const q = e.target.value.toLocaleLowerCase('tr');
    document.querySelectorAll('#games-table tr').forEach(r => r.hidden = !r.dataset.search.includes(q));
  });
  document.querySelector('#new-game')?.addEventListener('click', () => {
    document.querySelector('#game-form').reset();
    open('game-form');
  });
  document.querySelectorAll('.edit-game').forEach(b => b.addEventListener('click', () => {
    const game = JSON.parse(b.dataset.game);
    const form = document.querySelector('#game-form');
    Object.entries(game).forEach(([key,value]) => {
      const el = form.elements[key];
      if (!el) return;
      if (el.type === 'checkbox') el.checked = Boolean(Number(value));
      else if (key === 'supported_stores') el.value = Array.isArray(value) ? value.join(',') : value;
      else el.value = value ?? '';
    });
    open('game-form');
  }));
  document.querySelectorAll('.duplicate-game').forEach(b => b.addEventListener('click', async () => {
    if (!confirm('Bu oyunun ayarları pasif bir kopya olarak oluşturulsun mu?')) return;
    try { await api({action:'duplicate_game',game_id:Number(b.dataset.id)}); toast('Oyun kopyalandı'); location.reload(); }
    catch (error) { toast(error.message, true); }
  }));
  document.querySelectorAll('.toggle-game').forEach(b => b.addEventListener('click', async () => {
    const active = b.dataset.active === '1';
    if (!confirm(active ? 'Oyun tekrar aktifleştirilsin mi?' : 'Oyun pasife alınsın mı?')) return;
    try { await api({action:'set_game_status',game_id:Number(b.dataset.id),active}); toast(active ? 'Oyun aktifleştirildi' : 'Oyun pasife alındı'); location.reload(); }
    catch (error) { toast(error.message, true); }
  }));
  document.querySelector('#game-form')?.addEventListener('submit', async event => {
    event.preventDefault();
    const form = new FormData(event.target);
    const game = Object.fromEntries(form);
    game.id = Number(game.id || 0);
    game.translation_percent = Number(game.translation_percent);
    game.is_active = form.has('is_active');
    game.supported_stores = (game.supported_stores || 'manual').split(',').map(x => x.trim()).filter(Boolean);
    game.required_files = (game.required_files || '').split(/\r?\n/).map(x => x.trim()).filter(Boolean);
    game.optional_files = (game.optional_files || '').split(/\r?\n/).map(x => x.trim()).filter(Boolean);
    try { await api({action:'save_game',game}); toast('Oyun kaydedildi'); location.reload(); }
    catch (error) { toast(error.message, true); }
  });
  const gameImageKind=document.querySelector('#game-image-form select[name="kind"]');
  if(gameImageKind&&!gameImageKind.querySelector('option[value="icon"]'))gameImageKind.insertAdjacentHTML('beforeend','<option value="icon">Icon</option>');
  document.querySelector('#game-image-form')?.addEventListener('submit', async event => {
    event.preventDefault();
    const form = new FormData(event.target); form.append('action','upload_game_image');
    try {
      const response=await fetch('/admin/action',{method:'POST',headers:{'X-CSRF-Token':csrf},body:form});
      const json=await response.json();if(!response.ok||!json.ok)throw new Error(json.error?.message||'Görsel yüklenemedi');
      toast('Görsel optimize edilip kaydedildi');location.reload();
    } catch(error) { toast(error.message,true); }
  });
  document.querySelector('#delete-game-image')?.addEventListener('click', async () => {
    const form=document.querySelector('#game-image-form');const gameId=Number(form.elements.game_id.value);const kind=form.elements.kind.value;
    if(!gameId||!confirm('Seçilen görsel kaldırılsın mı?'))return;
    try{await api({action:'delete_game_image',game_id:gameId,kind});toast('Görsel kaldırıldı');location.reload();}catch(error){toast(error.message,true);}
  });
  document.querySelector('#patch-form')?.addEventListener('submit', async event => {
    event.preventDefault();
    const form = new FormData(event.target);
    form.append('action','create_patch');
    try {
      const response = await fetch('/admin/action',{method:'POST',headers:{'X-CSRF-Token':csrf},body:form});
      const json = await response.json();
      if (!response.ok || !json.ok) throw new Error(json.error?.message || 'Yükleme başarısız');
      document.querySelector('#builder-version-id').value = json.data.id;
      toast('Draft oluşturuldu. Version ID: ' + json.data.id);
    } catch (error) { toast(error.message, true); }
  });

  const types = ['COPY_FILE','COPY_DIRECTORY','REPLACE_FILE','DELETE_FILE','DELETE_DIRECTORY','CREATE_DIRECTORY','MOVE_FILE','RENAME_FILE','APPEND_FAT_DAT'];
  const escapeAttribute = value => String(value || '').replaceAll('&','&amp;').replaceAll('"','&quot;').replaceAll('<','&lt;');
  const addAction = (value = {}) => {
    const row = document.createElement('div');
    row.className = 'action-row';
    row.dataset.id = value.id || crypto.randomUUID();
    row.dataset.options = JSON.stringify(value.options || {});
    row.dataset.expectedHash = value.expected_sha256 || '';
    const options = types.map(type => '<option ' + (type === value.type ? 'selected' : '') + '>' + type + '</option>').join('');
    row.innerHTML = '<select class="action-type">' + options + '</select>' +
      '<input class="action-source" list="archive-files" placeholder="Archive/game source path" value="' + escapeAttribute(value.source) + '">' +
      '<input class="action-destination" placeholder="Game root relative destination" value="' + escapeAttribute(value.destination) + '">' +
      '<label class="switch"><input class="action-backup" type="checkbox" ' + (value.backup !== false ? 'checked' : '') + '> Backup</label>' +
      '<textarea class="action-options" aria-label="FAT/DAT kurulum seçenekleri" placeholder="FAT/DAT seçenekleri (JSON)">' + escapeAttribute(JSON.stringify(value.options || {}, null, 2)) + '</textarea>' +
      '<button class="icon-btn remove-action">Sil</button>';
    const updateOptionsVisibility = () => { row.querySelector('.action-options').hidden = row.querySelector('.action-type').value !== 'APPEND_FAT_DAT'; };
    row.querySelector('.action-type').addEventListener('change', updateOptionsVisibility);
    updateOptionsVisibility();
    row.querySelector('.remove-action').onclick = () => row.remove();
    document.querySelector('#action-list').append(row);
  };
  document.querySelector('#add-action')?.addEventListener('click', () => addAction());
  const loadBuilder = async id => {
    id=Number(id);if(!id)throw new Error('Geçerli Patch Version ID girin.');
    const data=await api({action:'load_patch_builder',version_id:id});
    document.querySelector('#builder-version-id').value=id;
    const tree=data.file_tree||[];
    document.querySelector('#archive-tree').innerHTML=tree.length?'<b>ZIP içeriği</b><ul>'+tree.map(item=>'<li><code>'+escapeAttribute(item.path)+'</code> <small>'+Number(item.size||0).toLocaleString('tr-TR')+' bayt</small></li>').join('')+'</ul>':'<small>Arşiv dosya ağacı boş.</small>';
    document.querySelector('#archive-files').innerHTML=tree.filter(item=>!item.directory).map(item=>'<option value="'+escapeAttribute(item.path)+'"></option>').join('');
    document.querySelector('#action-list').innerHTML='';
    (data.actions||[]).forEach(addAction);
    return data;
  };
  document.querySelector('#load-builder')?.addEventListener('click',async()=>{try{await loadBuilder(document.querySelector('#builder-version-id').value);toast('ZIP ağacı ve actionlar yüklendi');}catch(error){toast(error.message,true);}});
  const collectActions = () => [...document.querySelectorAll('.action-row')].map(row => ({
    id: row.dataset.id,
    type: row.querySelector('.action-type').value,
    source: row.querySelector('.action-source').value || null,
    destination: row.querySelector('.action-destination').value,
    backup: row.querySelector('.action-backup').checked,
    options: row.querySelector('.action-type').value === 'APPEND_FAT_DAT'
      ? JSON.parse(row.querySelector('.action-options').value || '{}') : JSON.parse(row.dataset.options || '{}'),
    expected_sha256: row.dataset.expectedHash || null
  }));
  document.querySelector('#save-actions')?.addEventListener('click', async () => {
    try { await api({action:'save_actions',version_id:Number(document.querySelector('#builder-version-id').value),actions:collectActions()}); toast('Action listesi kaydedildi'); }
    catch (error) { toast(error.message, true); }
  });
  const testManifest = async id => {
    const data = await api({action:'test_manifest',version_id:Number(id)});
    document.querySelector('#manifest-output').textContent = JSON.stringify(data,null,2);
    open('manifest');
    return data;
  };
  document.querySelector('#dry-manifest')?.addEventListener('click', async () => {
    try { await testManifest(document.querySelector('#builder-version-id').value); toast('Manifest testi tamamlandı'); }
    catch (error) { toast(error.message, true); }
  });
  document.querySelector('#load-manifest')?.addEventListener('click', async () => {
    try { await testManifest(document.querySelector('#manifest-version-id').value); }
    catch (error) { toast(error.message, true); }
  });
  document.querySelector('#publish-version')?.addEventListener('click', async () => {
    if (!confirm('Checklist başarılıysa bu sürüm kanalında yayınlanacak. Devam edilsin mi?')) return;
    try { await api({action:'publish_patch',version_id:Number(document.querySelector('#builder-version-id').value)}); toast('Patch yayınlandı'); location.reload(); }
    catch (error) { toast(error.message, true); }
  });
  document.querySelectorAll('.select-version').forEach(b => b.addEventListener('click', async () => {
    open('builder');
    try{await loadBuilder(b.dataset.id);}catch(error){toast(error.message,true);}
  }));
  document.querySelectorAll('.patch-status').forEach(b=>b.addEventListener('click',async()=>{const id=Number(document.querySelector('#builder-version-id').value);try{await api({action:'set_patch_status',version_id:id,status:b.dataset.status});toast('Patch durumu '+b.dataset.status+' olarak güncellendi');location.reload();}catch(error){toast(error.message,true);}}));
  document.querySelectorAll('#panel-patches tbody tr').forEach(row=>{if(!row.textContent.includes('ARCHIVED'))return;const select=row.querySelector('.select-version');if(!select)return;const rollback=document.createElement('button');rollback.className='icon-btn rollback-patch';rollback.textContent='Rollback';rollback.dataset.id=select.dataset.id;select.after(rollback);});
  document.querySelectorAll('.rollback-patch').forEach(button=>button.addEventListener('click',async()=>{if(!confirm('Bu sürüm yeniden aktif yayın olarak işaretlensin mi?'))return;try{await api({action:'rollback_patch',version_id:Number(button.dataset.id)});toast('Patch sürümü geri alındı ve yeniden yayınlandı');location.reload();}catch(error){toast(error.message,true);}}));
  const values = form => {
    const data=Object.fromEntries(new FormData(form));
    form.querySelectorAll('input[type="checkbox"]').forEach(input=>data[input.name]=input.checked);
    return data;
  };
  const fill = (form,data) => Object.entries(data).forEach(([key,value])=>{
    const input=form.elements[key];if(!input)return;
    if(input.type==='checkbox')input.checked=Boolean(Number(value));
    else if(input.type==='datetime-local')input.value=value?String(value).replace(' ','T').slice(0,16):'';
    else input.value=value??'';
  });
  document.querySelector('#category-form')?.addEventListener('submit',async event=>{event.preventDefault();try{await api({action:'save_category',category:values(event.currentTarget)});toast('Kategori kaydedildi');location.reload();}catch(error){toast(error.message,true);}});
  document.querySelectorAll('.edit-category').forEach(button=>button.addEventListener('click',()=>{fill(document.querySelector('#category-form'),JSON.parse(button.dataset.category));open('categories');}));
  document.querySelectorAll('.save-user').forEach(button=>button.addEventListener('click',async()=>{const row=button.closest('tr');try{await api({action:'update_user',user:{id:Number(row.dataset.userId),role:row.querySelector('.user-role').value,release_channel:row.querySelector('.user-channel').value,status:row.querySelector('.user-status').value}});toast('Kullanıcı yetkileri kaydedildi');}catch(error){toast(error.message,true);}}));
  document.querySelector('#subscription-form')?.addEventListener('submit',async event=>{event.preventDefault();try{await api({action:'save_subscription',subscription:values(event.currentTarget)});toast('Abonelik tanımlandı');location.reload();}catch(error){toast(error.message,true);}});
  document.querySelectorAll('.subscription-status').forEach(button=>button.addEventListener('click',async()=>{try{await api({action:'set_subscription_status',id:Number(button.dataset.id),status:button.dataset.status});toast('Abonelik durumu güncellendi');location.reload();}catch(error){toast(error.message,true);}}));
  document.querySelector('#announcement-form')?.addEventListener('submit',async event=>{event.preventDefault();try{await api({action:'save_announcement',announcement:values(event.currentTarget)});toast('Duyuru kaydedildi');location.reload();}catch(error){toast(error.message,true);}});
  document.querySelectorAll('.edit-announcement').forEach(button=>button.addEventListener('click',()=>{fill(document.querySelector('#announcement-form'),JSON.parse(button.dataset.announcement));open('announcements');}));
  document.querySelectorAll('.delete-announcement').forEach(button=>button.addEventListener('click',async()=>{if(!confirm('Duyuru silinsin mi?'))return;try{await api({action:'delete_announcement',id:Number(button.dataset.id)});toast('Duyuru silindi');location.reload();}catch(error){toast(error.message,true);}}));
  document.querySelector('#banner-form')?.addEventListener('submit',async event=>{event.preventDefault();const form=new FormData(event.currentTarget);form.append('action','save_banner');try{const response=await fetch('/admin/action',{method:'POST',headers:{'X-CSRF-Token':csrf},body:form});const json=await response.json();if(!response.ok||!json.ok)throw new Error(json.error?.message||'Banner yüklenemedi');toast('Banner yüklendi');location.reload();}catch(error){toast(error.message,true);}});
  document.querySelectorAll('.delete-banner').forEach(button=>button.addEventListener('click',async()=>{if(!confirm('Banner silinsin mi?'))return;try{await api({action:'delete_banner',id:Number(button.dataset.id)});toast('Banner silindi');location.reload();}catch(error){toast(error.message,true);}}));
  document.querySelector('#loader-version-form')?.addEventListener('submit',async event=>{event.preventDefault();const form=new FormData(event.currentTarget);form.append('action','create_loader_version');try{const response=await fetch('/admin/action',{method:'POST',headers:{'X-CSRF-Token':csrf},body:form});const json=await response.json();if(!response.ok||!json.ok)throw new Error(json.error?.message||'Loader paketi yüklenemedi');toast('Loader sürümü private storage alanına yüklendi');location.reload();}catch(error){toast(error.message,true);}});
  document.querySelector('#loader-config-form')?.addEventListener('submit', async event => {
    event.preventDefault();
    try { await api({action:'save_loader_config',config:Object.fromEntries(new FormData(event.target))}); toast('Remote loader config kaydedildi'); }
    catch (error) { toast(error.message, true); }
  });
  const previewUrls = new WeakMap();
  const releasePreview = preview => { const url=previewUrls.get(preview); if(url){URL.revokeObjectURL(url);previewUrls.delete(preview);} };
  const renderBrandingPreview = (form,file=null) => {
    const type=form.elements.background_type.value;
    const preview=form.querySelector('.branding-preview');
    releasePreview(preview);
    let source=file?URL.createObjectURL(file):preview.dataset.currentSrc;
    if(file)previewUrls.set(preview,source);
    if(type==='image'&&source)preview.innerHTML=`<img src="${source}" alt="Arka plan önizleme">`;
    else if(type==='video'&&source)preview.innerHTML=`<video src="${source}" poster="${preview.dataset.fallbackSrc||''}" muted loop autoplay playsinline controls></video>`;
    else preview.innerHTML='<div class="default-media-preview"><b>Animus varsayılan arka planı</b><small>Özel medya kullanılmıyor.</small></div>';
    form.querySelector('.fallback-upload').hidden=type!=='video';
    form.querySelector('.media-upload input').accept=type==='video'?'video/mp4,video/webm':'image/jpeg,image/png,image/webp';
    form.querySelector('.media-upload').classList.toggle('disabled',type==='default');
  };
  document.querySelectorAll('.branding-media-form').forEach(form=>{
    const type=form.elements.background_type,media=form.elements.media,overlay=form.elements.overlay;
    renderBrandingPreview(form);
    type.addEventListener('change',()=>renderBrandingPreview(form));
    overlay.addEventListener('input',()=>form.querySelector('.overlay-output').textContent='%'+overlay.value);
    media.addEventListener('change',()=>renderBrandingPreview(form,media.files?.[0]||null));
    const zone=form.querySelector('.media-upload');
    ['dragenter','dragover'].forEach(name=>zone.addEventListener(name,event=>{event.preventDefault();if(type.value!=='default')zone.classList.add('dragging');}));
    ['dragleave','drop'].forEach(name=>zone.addEventListener(name,event=>{event.preventDefault();zone.classList.remove('dragging');}));
    zone.addEventListener('drop',event=>{if(type.value==='default'||!event.dataTransfer?.files.length)return;media.files=event.dataTransfer.files;renderBrandingPreview(form,media.files[0]);});
    form.addEventListener('submit',async event=>{event.preventDefault();const data=new FormData(form);data.append('action','save_branding_media');try{const response=await fetch('/admin/action',{method:'POST',headers:{'X-CSRF-Token':csrf},body:data});const json=await response.json();if(!response.ok||!json.ok)throw new Error(json.error?.message||'Arka plan medyası kaydedilemedi');toast('Arka plan medyası yayınlandı');location.hash='loader';location.reload();}catch(error){toast(error.message,true);}});
    form.querySelector('.reset-branding').addEventListener('click',async()=>{if(!confirm('Bu arka plan Animus varsayılanına döndürülsün mü?'))return;try{await api({action:'reset_branding_media',slot:form.dataset.slot});toast('Varsayılan arka plan geri yüklendi');location.hash='loader';location.reload();}catch(error){toast(error.message,true);}});
  });
  window.addEventListener('beforeunload',()=>document.querySelectorAll('.branding-preview').forEach(releasePreview));

  // ---------------------------------------------------------------
  // Kalıcı silme akışı: önce sunucudan etki raporu, sonra onay.
  // ---------------------------------------------------------------
  const DELETE_ACTIONS = {
    game:            {action:'delete_game',            key:'game_id'},
    patch_version:   {action:'delete_patch_version',   key:'version_id'},
    loader_version:  {action:'delete_loader_version',  key:'id'},
    category:        {action:'delete_category',        key:'id'},
    announcement:    {action:'delete_announcement',    key:'id'},
    banner:          {action:'delete_banner',          key:'id'},
    subscription:    {action:'delete_subscription',    key:'id'},
    user:            {action:'delete_user',            key:'id'}
  };
  const DESCRIBABLE = new Set(['game','patch_version','loader_version','category','user']);
  const humanBytes = value => {
    const n = Number(value || 0);
    if (!n) return '0 B';
    const units = ['B','KB','MB','GB','TB'];
    const i = Math.min(units.length - 1, Math.floor(Math.log(n) / Math.log(1024)));
    return (n / Math.pow(1024, i)).toFixed(i ? 1 : 0) + ' ' + units[i];
  };
  const impactText = report => {
    const lines = [];
    (report.blocking || []).forEach(item => lines.push('! ' + item));
    const c = report.cascade || {};
    if (c.patch_versions !== undefined) lines.push('Silinecek yama sürümü: ' + c.patch_versions + ' (yayında: ' + (c.published_versions || 0) + ')');
    if (c.archive_bytes) lines.push('Karantinaya alınacak arşiv: ' + humanBytes(c.archive_bytes));
    if (c.package_bytes) lines.push('Karantinaya alınacak paket: ' + humanBytes(c.package_bytes));
    if (c.download_logs_detached) lines.push('İndirme kaydı korunacak (anonimleşir): ' + c.download_logs_detached);
    if (c.games_unlinked) lines.push('Kategorisi kaldırılacak oyun: ' + c.games_unlinked);
    if (c.replacement_version) lines.push('Kanal şu sürüme geri dönecek: ' + c.replacement_version);
    if (c.is_active_release && !c.replacement_version) lines.push('Bu kanalda yayında yama kalmayacak.');
    if (c.subscriptions) lines.push('Silinecek abonelik: ' + c.subscriptions);
    if (c.note) lines.push(c.note);
    return lines.join('\n');
  };
  const runDelete = async button => {
    const entity = button.dataset.entity;
    const id = Number(button.dataset.id);
    const label = button.dataset.label || ('#' + id);
    const map = DELETE_ACTIONS[entity];
    if (!map || !id) return;

    let force = false;
    let detail = '';
    if (DESCRIBABLE.has(entity)) {
      try {
        const report = await api({action:'describe_deletion', entity, id});
        force = Boolean(report.requires_force);
        const text = impactText(report);
        if (text) detail = '\n\n' + text;
      } catch (error) { toast(error.message, true); return; }
    }
    if (!confirm('KALICI SİLME\n\n"' + label + '" kalıcı olarak silinecek.' + detail + '\n\nDevam edilsin mi?')) return;
    if (force && !confirm('Bu kayıt yayında veya bağımlılığı var.\nYine de silmek istediğinize emin misiniz?')) return;

    button.disabled = true;
    try {
      const payload = {action: map.action, force};
      payload[map.key] = id;
      const data = await api(payload);
      toast('Silindi: ' + ((data && data.label) || label));
      location.reload();
    } catch (error) { button.disabled = false; toast(error.message, true); }
  };
  document.querySelectorAll('.delete-entity').forEach(button => button.addEventListener('click', () => runDelete(button)));

  // ---------------------------------------------------------------
  // Bakım / depolama paneli
  // ---------------------------------------------------------------
  const maintenanceOutput = document.querySelector('#maintenance-output');
  const renderStorage = status => {
    const grid = document.querySelector('#storage-stats');
    if (!grid || !status) return;
    const cards = [
      ['Bekleyen silme', status.pending],
      ['Başarısız silme', status.failed],
      ['Karantinadaki dosya', status.trash_files],
      ['Karantina boyutu', humanBytes(status.trash_bytes)]
    ];
    grid.innerHTML = cards.map(([label, value]) => '<article class="stat"><small>' + label + '</small><strong>' + value + '</strong></article>').join('');
  };
  const loadStorage = async () => {
    if (!document.querySelector('#storage-stats')) return;
    try { renderStorage(await api({action:'storage_status'})); } catch (error) { /* panel açılmamış olabilir */ }
  };
  document.querySelector('#refresh-storage')?.addEventListener('click', async () => {
    try { renderStorage(await api({action:'storage_status'})); toast('Depolama durumu güncellendi'); }
    catch (error) { toast(error.message, true); }
  });
  document.querySelectorAll('[data-maintenance]').forEach(button => button.addEventListener('click', async () => {
    if (button.dataset.confirm && !confirm(button.dataset.confirm)) return;
    const payload = {action: button.dataset.maintenance};
    if (button.dataset.days) payload.days = Number(document.querySelector('#' + button.dataset.days)?.value || 0);
    if (button.dataset.apply) payload.apply = true;
    button.disabled = true;
    try {
      const data = await api(payload);
      maintenanceOutput.textContent = JSON.stringify(data, null, 2);
      toast('İşlem tamamlandı');
      loadStorage();
    } catch (error) { maintenanceOutput.textContent = 'Hata: ' + error.message; toast(error.message, true); }
    finally { button.disabled = false; }
  }));
  if (location.hash.slice(1) === 'maintenance') loadStorage();
  document.querySelector('[data-panel="maintenance"]')?.addEventListener('click', loadStorage);

  addAction({type:'REPLACE_FILE',source:'files/translation.dat',destination:'Localization/translation.dat',backup:true});
})();


