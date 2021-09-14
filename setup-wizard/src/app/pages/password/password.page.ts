import { Component, Input } from '@angular/core'
import { LoadingController, ModalController } from '@ionic/angular'
import { ApiService, DiskInfo } from 'src/app/services/api/api.service'

@Component({
  selector: 'app-password',
  templateUrl: 'password.page.html',
  styleUrls: ['password.page.scss'],
})
export class PasswordPage {
  @Input() recoveryDrive: DiskInfo
  @Input() storageDrive: DiskInfo

  pwError = ''
  password = ''
  unmasked1 = false

  verError = ''
  passwordVer = ''
  unmasked2 = false

  hasData: boolean

  constructor(
    private modalController: ModalController,
    private apiService: ApiService,
    private loadingCtrl: LoadingController,
  ) {}

  ngOnInit() {
    if (this.storageDrive && this.storageDrive.partitions.find(p => p.used)) {
      this.hasData = true
    }
  }

  async verifyPw () {
    if (!this.recoveryDrive) this.pwError = 'No recovery drive' // unreachable
    const loader = await this.loadingCtrl.create({
      message: 'Verifying Password'
    })
    await loader.present()

    try {
      const isCorrectPassword = await this.apiService.verifyRecoveryPassword(this.recoveryDrive.logicalname, this.password)
      if(isCorrectPassword) {
        this.modalController.dismiss({ password: this.password })
      } else {
        this.pwError = "Incorrect password provided"
      }
    } catch (e) {
      this.pwError = 'Error connecting to Embassy'
    } finally {
      loader.dismiss()
    }
  }

  async submitPw () {
    this.validate()
    if (this.password !== this.passwordVer) {
      this.verError="*passwords do not match"
    }

    if(this.pwError || this.verError) return
    this.modalController.dismiss({ password: this.password })
  }

  validate () {
    if(!!this.recoveryDrive) return this.pwError = ''

    if (this.passwordVer) {
      this.checkVer()
    }

    if (this.password.length < 12) {
      this.pwError="*password must be 12 characters or greater"
    } else {
      this.pwError = ''
    }
  }

  checkVer () {
    this.verError = this.password !== this.passwordVer ? "*passwords do not match" : ''
  }

  cancel () {
    this.modalController.dismiss()
  }
}
