import * as THREE from 'three'

// 描画先の要素
const appElement = document.querySelector<HTMLDivElement>('#output')!

function init() {
  // シーンの作成（物体、光源の表示、保持をするオブジェクト）
  const scene = new THREE.Scene()

  /**
   * カメラの作成（何が見えるかの設定）
   * @param fovy - カメラの角度
   * @param aspect - カメラのアスペクト比
   * @param near - 深度(手前)
   * @param far - 深度(奥)
  **/
  const camera = new THREE.PerspectiveCamera(45, window.innerWidth / window.innerHeight, 0.1, 1000)

  // レンダラーの作成（cameraオブジェクトの角度に基づき、sceneオブジェクトがどう見えるか計算してくれる）
  const renderer = new THREE.WebGLRenderer()

  //背景色
  renderer.setClearColor(new THREE.Color(0x000000))

  //sceneの大きさの通知
  renderer.setSize(window.innerWidth, window.innerHeight);

  //デバイスの表示調整
  renderer.setPixelRatio(window.devicePixelRatio);

  // sphere（球）の作成
  const sphereGeometry = new THREE.SphereGeometry(10, 20, 20);
  const sphereMaterial = new THREE.MeshBasicMaterial({color: 0x00aaff, wireframe: true});
  const sphere = new THREE.Mesh(sphereGeometry, sphereMaterial);

  // sphereの配置
  sphere.position.x = 0;
  sphere.position.y = 10;
  sphere.position.z = 1;

  // sphereをsceneに追加して表示する
  scene.add(sphere);

  // カメラを中心に配置する
  camera.position.x =-30;
  camera.position.y = 40;
  camera.position.z = 90;
  camera.lookAt(scene.position);

  // HTMLにレンダラーの出力を追加する
  appElement.appendChild(renderer.domElement);

  // renderに指示 cameraをsceneに渡して、表示する
  renderer.render(scene, camera);
}

window.onload = init;
